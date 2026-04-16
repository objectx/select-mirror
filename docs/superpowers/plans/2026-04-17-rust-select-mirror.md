# select-mirror Rust CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI that probes a list of Ubuntu mirror URLs in parallel and prints the fastest one to stdout.

**Architecture:** One `std::thread` per mirror fires a GET request via `ureq` and sends the elapsed time (or failure) over an `mpsc` channel. The main thread collects all results, picks the minimum elapsed time, and exits non-zero if every mirror failed.

**Tech Stack:** Rust 2021, `clap` 4 (derive), `ureq` 2, `assert_cmd` 2 (dev)

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Package manifest; declares `clap`, `ureq`, `assert_cmd` |
| `src/main.rs` | CLI parsing, `find_best` pure fn, `probe` fn, threading, output |
| `tests/cli.rs` | Integration tests: error-exit case, happy-path with mock server |

---

### Task 1: Initialize Cargo project

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "select-mirror"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "select-mirror"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
ureq = "2"

[dev-dependencies]
assert_cmd = "2"
```

- [ ] **Step 2: Create minimal `src/main.rs` so it compiles**

```rust
fn main() {}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build
```

Expected: no errors, `target/debug/select-mirror` created.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "chore: init Rust project with clap and ureq"
```

---

### Task 2: Implement and test `find_best`

**Files:**
- Modify: `src/main.rs`

`find_best` is a pure function: given a slice of `(mirror_url, Option<elapsed_secs>)`, return the mirror with the smallest elapsed time, or `None` if all failed.

- [ ] **Step 1: Write the failing tests**

Replace `src/main.rs` with:

```rust
fn find_best(results: &[(String, Option<f64>)]) -> Option<&str> {
    todo!()
}

fn main() {}

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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test
```

Expected: all four tests fail with "not yet implemented".

- [ ] **Step 3: Implement `find_best`**

Replace the `todo!()` body:

```rust
fn find_best(results: &[(String, Option<f64>)]) -> Option<&str> {
    results
        .iter()
        .filter_map(|(mirror, elapsed)| elapsed.map(|t| (mirror.as_str(), t)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(mirror, _)| mirror)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test
```

Expected: 4 tests pass, 0 failures.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement find_best selection logic with tests"
```

---

### Task 3: Implement CLI argument parsing

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write a failing test for required mirror argument**

Add to `tests/cli.rs` (create the file):

```rust
use assert_cmd::Command;

#[test]
fn exits_nonzero_with_no_mirrors() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .assert()
        .failure();
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --test cli exits_nonzero_with_no_mirrors
```

Expected: FAIL (binary currently does nothing, exits 0).

- [ ] **Step 3: Add CLI parsing to `src/main.rs`**

Add the `Args` struct and wire up `main`. Full `src/main.rs` at this point:

```rust
use clap::Parser;

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

fn find_best(results: &[(String, Option<f64>)]) -> Option<&str> {
    results
        .iter()
        .filter_map(|(mirror, elapsed)| elapsed.map(|t| (mirror.as_str(), t)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(mirror, _)| mirror)
}

fn main() {
    let _args = Args::parse();
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
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: 5 tests pass (4 unit + 1 CLI).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs tests/cli.rs
git commit -m "feat: add CLI argument parsing with clap"
```

---

### Task 4: Implement probing and threading

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add the integration test for all-mirrors-fail first**

Add to `tests/cli.rs`:

```rust
#[test]
fn exits_nonzero_when_all_mirrors_fail() {
    // Ports 1 and 2 are reserved and will refuse connections immediately.
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args(["http://127.0.0.1:1", "http://127.0.0.1:2"])
        .args(["--timeout", "1"])
        .assert()
        .failure();
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test --test cli exits_nonzero_when_all_mirrors_fail
```

Expected: FAIL (main currently does nothing after parsing).

- [ ] **Step 3: Implement `probe` and full `main` in `src/main.rs`**

Replace `main` and add `probe`. Full `src/main.rs`:

```rust
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
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: all tests pass including `exits_nonzero_when_all_mirrors_fail`. The test with bad ports may take up to 2 seconds (timeout=1 × 2 threads).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs tests/cli.rs
git commit -m "feat: implement parallel mirror probing with threads and ureq"
```

---

### Task 5: Happy-path integration test with mock HTTP server

**Files:**
- Modify: `tests/cli.rs`

The test spins up a real TCP listener that responds with HTTP 200. We read the incoming request bytes before writing the response so ureq doesn't see a dropped connection.

- [ ] **Step 1: Add the happy-path test to `tests/cli.rs`**

Full `tests/cli.rs` after adding the new test:

```rust
use assert_cmd::Command;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn start_mock_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf); // drain the request
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            }
        }
    });
    port
}

#[test]
fn exits_nonzero_with_no_mirrors() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn exits_nonzero_when_all_mirrors_fail() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args(["http://127.0.0.1:1", "http://127.0.0.1:2"])
        .args(["--timeout", "1"])
        .assert()
        .failure();
}

#[test]
fn outputs_fastest_mirror_url() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror.as_str()
    );
}

#[test]
fn fastest_of_two_mirrors_wins() {
    // Start two mock servers; both succeed, tool must output one of them.
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        winner == mirror_a || winner == mirror_b,
        "unexpected winner: {winner}"
    );
}
```

- [ ] **Step 2: Run all tests**

```bash
cargo test
```

Expected: all 8 tests pass (4 unit + 4 integration).

- [ ] **Step 3: Commit**

```bash
git add tests/cli.rs
git commit -m "test: add happy-path integration tests with mock HTTP server"
```

---

### Task 6: Final build verification

- [ ] **Step 1: Build release binary**

```bash
cargo build --release
```

Expected: `target/release/select-mirror` created with no warnings.

- [ ] **Step 2: Smoke-test against a real mirror**

```bash
target/release/select-mirror \
  "http://archive.ubuntu.com/ubuntu" \
  "http://jp.archive.ubuntu.com/ubuntu" \
  --timeout 5
```

Expected: timing lines on stderr, one URL on stdout.

- [ ] **Step 3: Run full test suite one final time**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: release build verified"
```
