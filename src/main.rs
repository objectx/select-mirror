use clap::Parser;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CACHE_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    version: u32,
    mirror: String,
    elapsed_ms: u64,
    probe_path: String,
    recorded_at: u64,
}

impl CacheEntry {
    fn new(mirror: &str, elapsed_ms: u64, probe_path: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            version: CACHE_VERSION,
            mirror: mirror.to_string(),
            elapsed_ms,
            probe_path: probe_path.to_string(),
            recorded_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

fn load_cache(path: &str) -> Option<CacheEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let entry: CacheEntry = match serde_json::from_str(&content) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("warning: cache file is malformed, ignoring");
            return None;
        }
    };
    if entry.version != CACHE_VERSION {
        eprintln!(
            "warning: cache file has unsupported version {}, ignoring",
            entry.version
        );
        return None;
    }
    Some(entry)
}

fn save_cache(path: &str, entry: &CacheEntry) {
    let json = match serde_json::to_string_pretty(&entry) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("warning: failed to serialize cache: {}", e);
            return;
        }
    };
    let tmp_path = format!("{}.tmp.{}", path, std::process::id());
    if let Err(e) = std::fs::write(&tmp_path, json.as_bytes()) {
        eprintln!("warning: failed to write cache: {}", e);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        eprintln!("warning: failed to finalize cache: {}", e);
        let _ = std::fs::remove_file(&tmp_path);
    }
}

#[derive(Parser)]
#[command(about = "Select the fastest Ubuntu mirror")]
struct Args {
    /// One or more mirror base URLs to probe
    #[arg(required = true)]
    mirrors: Vec<String>,

    /// Path appended to each mirror URL for probing
    #[arg(long, default_value = "/ubuntu/dists/noble/Release")]
    probe_path: String,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 3u64)]
    timeout: u64,

    /// Response-time threshold in milliseconds to qualify as "fast" (e.g. 500 = 0.5 s)
    #[arg(long, default_value_t = 500u64)]
    fast_threshold: u64,

    /// Stop after this many mirrors respond within --fast-threshold (must be >= 1)
    #[arg(long, default_value_t = 3usize, value_parser = parse_fast_count)]
    fast_count: usize,

    /// Path to the cache file for persisting the selected mirror
    #[arg(long, default_value = ".selected-mirror.json")]
    cache_file: String,

    /// Skip reading the cache and re-probe all mirrors (still writes the result)
    #[arg(long)]
    no_cache: bool,
}

fn probe(mirror: &str, probe_path: &str, timeout_secs: u64) -> Option<f64> {
    let url = format!("{}{}", mirror, probe_path);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .build()
        .into();
    let start = Instant::now();
    agent
        .get(&url)
        .call()
        .ok()
        .map(|_| start.elapsed().as_secs_f64())
}

fn find_best(results: &[(String, Option<f64>)]) -> Option<&str> {
    results
        .iter()
        .filter_map(|(mirror, elapsed)| elapsed.map(|t| (mirror.as_str(), t)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(mirror, _)| mirror)
}

fn parse_fast_count(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|_| format!("'{}' is not a valid integer", s))?;
    if n == 0 {
        Err("--fast-count must be >= 1".to_string())
    } else {
        Ok(n)
    }
}

fn secs_to_ms(secs: f64) -> u64 {
    (secs * 1000.0) as u64
}

fn main() {
    let args = Args::parse();

    // Cache-hit short-circuit
    if !args.no_cache {
        if let Some(entry) = load_cache(&args.cache_file) {
            if entry.probe_path == args.probe_path && args.mirrors.contains(&entry.mirror) {
                match probe(&entry.mirror, &args.probe_path, args.timeout) {
                    None => {
                        eprintln!("  {}: unreachable, re-probing all", entry.mirror);
                    }
                    Some(e) => {
                        let threshold_secs = args.fast_threshold as f64 / 1000.0;
                        if e < threshold_secs {
                            eprintln!("  {}: {:.3}s (cached)", entry.mirror, e);
                            save_cache(
                                &args.cache_file,
                                &CacheEntry::new(&entry.mirror, secs_to_ms(e), &args.probe_path),
                            );
                            println!("{}", entry.mirror);
                            return;
                        }
                        // slow — fall through to probe-all silently
                    }
                }
            }
        }
    }

    // Probe-all flow (unchanged)
    let (tx, rx) = mpsc::channel();
    for mirror in &args.mirrors {
        let tx = tx.clone();
        let mirror = mirror.clone();
        let probe_path = args.probe_path.clone();
        let timeout = args.timeout;
        thread::spawn(move || {
            let elapsed = probe(&mirror, &probe_path, timeout);
            let _ = tx.send((mirror, elapsed));
        });
    }
    drop(tx);

    let threshold_secs = args.fast_threshold as f64 / 1000.0;
    let mut fast_count_seen: usize = 0;
    let mut results: Vec<(String, Option<f64>)> = Vec::new();

    for (mirror, elapsed) in rx {
        let label = elapsed
            .map(|e| format!("{:.3}s", e))
            .unwrap_or_else(|| "failed".to_string());
        eprintln!("  {}: {}", mirror, label);
        results.push((mirror, elapsed));
        if elapsed.is_some_and(|e| e < threshold_secs) {
            fast_count_seen += 1;
            if fast_count_seen >= args.fast_count {
                break;
            }
        }
    }

    match find_best(&results) {
        Some(best) => {
            let elapsed_ms = results
                .iter()
                .find(|(m, _)| m.as_str() == best)
                .and_then(|(_, e)| *e)
                .map(secs_to_ms)
                .unwrap_or(0);
            save_cache(&args.cache_file, &CacheEntry::new(best, elapsed_ms, &args.probe_path));
            println!("{}", best);
        }
        None => {
            eprintln!("Error: all mirrors failed or timed out");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_best_returns_fastest() {
        let results = vec![
            ("http://a.example.com".to_string(), Some(1.5)),
            ("http://b.example.com".to_string(), Some(0.3)),
            ("http://c.example.com".to_string(), Some(2.1)),
        ];
        assert_eq!(find_best(&results), Some("http://b.example.com"));
    }

    #[test]
    fn find_best_skips_failed_mirrors() {
        let results = vec![
            ("http://a.example.com".to_string(), None),
            ("http://b.example.com".to_string(), Some(0.5)),
        ];
        assert_eq!(find_best(&results), Some("http://b.example.com"));
    }

    #[test]
    fn find_best_returns_none_when_all_failed() {
        let results = vec![
            ("http://a.example.com".to_string(), None),
            ("http://b.example.com".to_string(), None),
        ];
        assert_eq!(find_best(&results), None);
    }

    #[test]
    fn find_best_returns_none_for_empty_input() {
        let results: Vec<(String, Option<f64>)> = vec![];
        assert_eq!(find_best(&results), None);
    }

    #[test]
    fn parse_fast_count_rejects_zero() {
        assert!(parse_fast_count("0").is_err());
    }

    #[test]
    fn parse_fast_count_rejects_non_integer() {
        assert!(parse_fast_count("abc").is_err());
    }

    #[test]
    fn parse_fast_count_accepts_one() {
        assert_eq!(parse_fast_count("1").unwrap(), 1);
    }

    #[test]
    fn parse_fast_count_accepts_valid_count() {
        assert_eq!(parse_fast_count("3").unwrap(), 3);
    }

    #[test]
    fn load_cache_returns_none_for_missing_file() {
        assert!(load_cache("/nonexistent/path/select-mirror-test-cache.json").is_none());
    }

    #[test]
    fn load_cache_returns_none_for_malformed_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.json").to_str().unwrap().to_string();
        std::fs::write(&path, b"not json {{{{").unwrap();
        assert!(load_cache(&path).is_none());
    }

    #[test]
    fn load_cache_returns_none_for_unknown_version() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("v99.json").to_str().unwrap().to_string();
        std::fs::write(
            &path,
            br#"{"version":99,"mirror":"http://x.com","elapsed_ms":100,"probe_path":"/","recorded_at":0}"#,
        )
        .unwrap();
        assert!(load_cache(&path).is_none());
    }

    #[test]
    fn save_cache_and_load_cache_round_trip() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("cache.json").to_str().unwrap().to_string();
        let entry_in = CacheEntry {
            version: CACHE_VERSION,
            mirror: "http://example.com".to_string(),
            elapsed_ms: 250,
            probe_path: "/probe".to_string(),
            recorded_at: before,
        };
        save_cache(&path, &entry_in);
        let entry_out = load_cache(&path).expect("should load saved cache");
        assert_eq!(entry_out.mirror, "http://example.com");
        assert_eq!(entry_out.elapsed_ms, 250);
        assert_eq!(entry_out.probe_path, "/probe");
        assert_eq!(entry_out.version, CACHE_VERSION);
        assert_eq!(entry_out.recorded_at, before);
    }

    #[test]
    fn save_cache_is_non_fatal_for_unwritable_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("nonexistent-subdir")
            .join("cache.json")
            .to_str()
            .unwrap()
            .to_string();
        let entry = CacheEntry {
            version: CACHE_VERSION,
            mirror: "http://example.com".to_string(),
            elapsed_ms: 100,
            probe_path: "/".to_string(),
            recorded_at: 0,
        };
        // must not panic
        save_cache(&path, &entry);
    }
}
