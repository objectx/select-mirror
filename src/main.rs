use clap::Parser;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

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
}

fn probe(mirror: &str, probe_path: &str, timeout_secs: u64) -> Option<f64> {
    let url = format!("{}{}", mirror, probe_path);
    let start = Instant::now();
    ureq::get(&url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
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

fn main() {
    let args = Args::parse();
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
        Some(best) => println!("{}", best),
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
}
