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
            tx.send((mirror, elapsed)).unwrap();
        });
    }
    drop(tx);

    let mut results: Vec<(String, Option<f64>)> = Vec::new();
    for (mirror, elapsed) in rx {
        let label = elapsed
            .map(|e| format!("{:.3}s", e))
            .unwrap_or_else(|| "failed".to_string());
        eprintln!("  {}: {}", mirror, label);
        results.push((mirror, elapsed));
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
}
