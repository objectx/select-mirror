# Cache Selected Mirror Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist the most recently selected mirror to `.selected-mirror.json` so that consecutive invocations with a stable network return the same mirror, preventing unnecessary Docker layer cache busts.

**Architecture:** On every invocation, `main` attempts to read and validate a JSON cache file, then probes only the cached mirror. If it responds within `--fast-threshold`, the tool short-circuits and returns it; otherwise it falls through to the existing parallel probe-all flow unchanged. The winning mirror is always written back to the cache (atomic `tmp + rename`). Two new CLI flags — `--cache-file` and `--no-cache` — control the cache path and read-bypass behaviour. All logic stays in `src/main.rs`.

**Tech Stack:** Rust, clap (derive), serde + serde_json (new), std::fs::rename (atomic write), assert_cmd + tempfile (integration tests)

---

## Files

- Modify: `Cargo.toml` — add `serde`, `serde_json` dependencies; add `tempfile` dev-dependency
- Modify: `src/main.rs` — add `CacheEntry` struct, `load_cache`, `save_cache` functions; add `--cache-file` / `--no-cache` to `Args`; add cache-hit short-circuit and cache-write to `main`; add unit tests
- Modify: `tests/cli.rs` — add `tempfile::TempDir` import; add `--cache-file` to all existing tests; add 9 new cache integration tests
- Modify: `.gitignore` — add `.selected-mirror.json`
- Modify: `README.md` — document new flags and caching behaviour
- Modify: `CLAUDE.md` — update architecture section

---

### Task 1: Add serde deps and implement cache I/O functions

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`

- [ ] **Step 1: Add serde, serde_json, and tempfile to Cargo.toml**

Replace the `[dependencies]` and `[dev-dependencies]` blocks in `Cargo.toml` with:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
ureq = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
```

- [ ] **Step 2: Verify the project compiles**

```bash
cargo build
```

Expected: compiles with no errors or warnings.

- [ ] **Step 3: Add `CacheEntry` struct and `load_cache` / `save_cache` functions to `src/main.rs`**

Insert the following block immediately after the `use clap::Parser;` line (before the `#[derive(Parser)]` `Args` struct):

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    version: u32,
    mirror: String,
    elapsed_ms: u64,
    probe_path: String,
    recorded_at: u64,
}

fn load_cache(path: &str) -> Option<CacheEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&content).ok()?;
    if entry.version != 1 {
        return None;
    }
    Some(entry)
}

fn save_cache(path: &str, mirror: &str, elapsed_ms: u64, probe_path: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let entry = CacheEntry {
        version: 1,
        mirror: mirror.to_string(),
        elapsed_ms,
        probe_path: probe_path.to_string(),
        recorded_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
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
```

- [ ] **Step 4: Add unit tests for `load_cache` and `save_cache`**

Append the following tests inside the existing `#[cfg(test)] mod tests { ... }` block in `src/main.rs`, after the last existing test:

```rust
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
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("cache.json").to_str().unwrap().to_string();
        save_cache(&path, "http://example.com", 250, "/probe");
        let entry = load_cache(&path).expect("should load saved cache");
        assert_eq!(entry.mirror, "http://example.com");
        assert_eq!(entry.elapsed_ms, 250);
        assert_eq!(entry.probe_path, "/probe");
        assert_eq!(entry.version, 1);
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
        // must not panic
        save_cache(&path, "http://example.com", 100, "/");
    }
```

- [ ] **Step 5: Run all tests to verify unit tests pass**

```bash
cargo test
```

Expected: all tests pass. You should see the 5 new cache unit tests in the output.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "feat: add CacheEntry struct with load_cache and save_cache"
```

---

### Task 2: Wire cache flags and flow into `main`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `--cache-file` and `--no-cache` to `Args`**

In `src/main.rs`, replace the entire `Args` struct with:

```rust
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
```

- [ ] **Step 2: Replace the entire `main` function with the cache-aware version**

In `src/main.rs`, replace the entire `fn main()` with:

```rust
fn main() {
    let args = Args::parse();

    // Cache-hit short-circuit
    if !args.no_cache {
        if let Some(entry) = load_cache(&args.cache_file) {
            if entry.probe_path == args.probe_path && args.mirrors.contains(&entry.mirror) {
                if let Some(e) = probe(&entry.mirror, &args.probe_path, args.timeout) {
                    let threshold_secs = args.fast_threshold as f64 / 1000.0;
                    if e < threshold_secs {
                        eprintln!("  {}: {:.3}s (cached)", entry.mirror, e);
                        save_cache(
                            &args.cache_file,
                            &entry.mirror,
                            (e * 1000.0) as u64,
                            &args.probe_path,
                        );
                        println!("{}", entry.mirror);
                        return;
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
                .map(|e| (e * 1000.0) as u64)
                .unwrap_or(0);
            save_cache(&args.cache_file, best, elapsed_ms, &args.probe_path);
            println!("{}", best);
        }
        None => {
            eprintln!("Error: all mirrors failed or timed out");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 3: Run all tests to verify everything still passes**

```bash
cargo test
```

Expected: all existing tests pass. (Integration tests will now write `.selected-mirror.json` in CWD — this is temporary and will be fixed in Task 3.)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --cache-file / --no-cache flags and cache-hit short-circuit to main"
```

---

### Task 3: Complete integration test suite

**Files:**
- Modify: `tests/cli.rs`

- [ ] **Step 1: Replace the entire `tests/cli.rs` with the complete updated version**

```rust
use assert_cmd::Command;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn start_slow_server(delay_ms: u64) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            thread::sleep(Duration::from_millis(delay_ms));
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        }
    });
    port
}

fn start_mock_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        }
    });
    port
}

fn cache_file(dir: &TempDir) -> String {
    dir.path().join("cache.json").to_str().unwrap().to_string()
}

// ─── Existing tests (with isolated --cache-file) ──────────────────────────────

#[test]
fn exits_nonzero_with_no_mirrors() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn exits_nonzero_when_all_mirrors_fail() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args(["http://127.0.0.1:1", "http://127.0.0.1:2"])
        .args(["--timeout", "1"])
        .args(["--cache-file", &cache_file(&dir)])
        .assert()
        .failure();
}

#[test]
fn outputs_fastest_mirror_url() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cache_file(&dir)])
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
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);
    let dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cache_file(&dir)])
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

#[test]
fn early_exit_when_fast_count_met() {
    let fast_a = format!("http://127.0.0.1:{}", start_mock_server());
    let fast_b = format!("http://127.0.0.1:{}", start_mock_server());
    let slow = format!("http://127.0.0.1:{}", start_slow_server(3000));
    let dir = TempDir::new().unwrap();

    let start = std::time::Instant::now();
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&fast_a, &fast_b, &slow])
        .args(["--probe-path", "/"])
        .args(["--fast-count", "2"])
        .args(["--fast-threshold", "500"])
        .args(["--timeout", "5"])
        .args(["--cache-file", &cache_file(&dir)])
        .output()
        .unwrap();
    let elapsed = start.elapsed();

    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        winner == fast_a || winner == fast_b,
        "winner {winner} should be one of the fast mirrors"
    );
    assert!(
        elapsed.as_millis() < 2500,
        "expected early exit in under 2.5s, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn fast_count_exceeds_mirror_count_still_succeeds() {
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);
    let dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .args(["--fast-count", "10"])
        .args(["--fast-threshold", "500"])
        .args(["--cache-file", &cache_file(&dir)])
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

// ─── Cache tests ──────────────────────────────────────────────────────────────

#[test]
fn first_run_writes_cache_file() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .assert()
        .success();

    assert!(
        std::path::Path::new(&cf).exists(),
        "cache file should be written after first run"
    );
    let content = std::fs::read_to_string(&cf).unwrap();
    assert!(
        content.contains(&mirror),
        "cache file should contain the chosen mirror URL"
    );
}

#[test]
fn cache_hit_uses_cached_mirror() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // First run: writes cache
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .assert()
        .success();

    // Second run: should get a cache hit
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror.as_str()
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("(cached)"),
        "expected '(cached)' in stderr on second run, got: {stderr}"
    );
}

#[test]
fn cache_miss_when_probe_path_differs() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Write cache with /path-a
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/path-a"])
        .args(["--cache-file", &cf])
        .assert()
        .success();

    // Run with /path-b: should not be a cache hit
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/path-b"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit when probe-path differs, got: {stderr}"
    );
}

#[test]
fn cache_miss_when_cached_mirror_not_in_argv() {
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Write cache selecting mirror_a
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .assert()
        .success();

    // Run with only mirror_b: cached mirror_a is absent from argv
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_b])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror_b.as_str()
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit when cached mirror absent from argv, got: {stderr}"
    );
}

#[test]
fn cache_miss_for_malformed_json() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    std::fs::write(&cf, b"not valid json {{{{").unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit for malformed JSON, got: {stderr}"
    );
}

#[test]
fn cache_miss_for_unknown_version() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    std::fs::write(
        &cf,
        format!(
            r#"{{"version":999,"mirror":"{}","elapsed_ms":100,"probe_path":"/","recorded_at":0}}"#,
            mirror
        )
        .as_bytes(),
    )
    .unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit for unknown version, got: {stderr}"
    );
}

#[test]
fn cache_miss_when_cached_probe_is_slow() {
    let fast_port = start_mock_server();
    let slow_port = start_slow_server(2000);
    let fast_mirror = format!("http://127.0.0.1:{}", fast_port);
    let slow_mirror = format!("http://127.0.0.1:{}", slow_port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Seed cache pointing to the slow mirror (with a falsely-low elapsed_ms)
    std::fs::write(
        &cf,
        format!(
            r#"{{"version":1,"mirror":"{}","elapsed_ms":50,"probe_path":"/","recorded_at":0}}"#,
            slow_mirror
        )
        .as_bytes(),
    )
    .unwrap();

    // slow_mirror takes 2 s; threshold is 500 ms → probe fails the gate, falls through
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&fast_mirror, &slow_mirror])
        .args(["--probe-path", "/"])
        .args(["--fast-threshold", "500"])
        .args(["--timeout", "5"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        fast_mirror.as_str(),
        "fast mirror should win after slow cached probe falls through"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit, got: {stderr}"
    );
}

#[test]
fn no_cache_flag_skips_read_but_still_writes() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Seed a valid cache that would otherwise produce a hit
    std::fs::write(
        &cf,
        format!(
            r#"{{"version":1,"mirror":"{}","elapsed_ms":10,"probe_path":"/","recorded_at":0}}"#,
            mirror
        )
        .as_bytes(),
    )
    .unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .args(["--no-cache"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror.as_str()
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit with --no-cache, got: {stderr}"
    );

    // Cache file should still contain the winner (was written by probe-all path)
    let cache_content = std::fs::read_to_string(&cf).unwrap();
    assert!(
        cache_content.contains(&mirror),
        "cache file should be updated even under --no-cache, got: {cache_content}"
    );
}

#[test]
fn cache_write_failure_is_non_fatal() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();

    // Parent directory of the cache path does not exist → write will fail
    let cf = dir
        .path()
        .join("no-such-subdir")
        .join("cache.json")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "should exit 0 even when cache write fails"
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror.as_str()
    );
}
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all 15 tests pass (6 existing + 9 new cache tests). If `cache_miss_when_cached_probe_is_slow` is flaky due to system load, re-run once; the timing margin (2 s slow vs 500 ms threshold) is intentionally wide.

- [ ] **Step 3: Commit**

```bash
git add tests/cli.rs
git commit -m "test: add cache integration tests; isolate existing tests with --cache-file"
```

---

### Task 4: Repo hygiene and docs

**Files:**
- Modify: `.gitignore`
- Modify: `README.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add `.selected-mirror.json` to `.gitignore`**

Append to `.gitignore`:

```
.selected-mirror.json
```

- [ ] **Step 2: Update `README.md` with new flags and caching behaviour**

Replace the `## Usage` section (the code block + the options list + surrounding prose) with:

```markdown
## Usage

```
select-mirror [OPTIONS] <MIRRORS>...

Arguments:
  <MIRRORS>...  One or more mirror base URLs to probe

Options:
      --probe-path <PROBE_PATH>
          Path appended to each mirror URL [default: /ubuntu/dists/noble/Release]
      --timeout <TIMEOUT>
          Request timeout in seconds [default: 3]
      --fast-threshold <FAST_THRESHOLD>
          Response-time threshold in milliseconds to qualify as "fast" [default: 500]
      --fast-count <FAST_COUNT>
          Stop after this many mirrors respond within --fast-threshold [default: 3]
      --cache-file <CACHE_FILE>
          Path to the cache file for persisting the selected mirror
          [default: .selected-mirror.json]
      --no-cache
          Skip reading the cache and re-probe all mirrors (still writes the result)
  -h, --help
          Print help
```

## Caching

On each successful run the tool writes the chosen mirror to `.selected-mirror.json` (in the current directory by default). On the next invocation it probes only that cached mirror first; if it responds within `--fast-threshold` it is returned immediately without probing any other mirror.

This makes consecutive invocations return the same mirror as long as the network is stable — useful when the output drives a Docker `apt` mirror layer that you do not want to rebuild unnecessarily.

Use `--no-cache` to force a fresh probe while still updating the cache for the next run. Use `--cache-file /dev/null` to disable caching entirely (reads nothing, discards the write).
```

- [ ] **Step 3: Update `CLAUDE.md` architecture section**

In `CLAUDE.md`, replace the bullet list inside the `## Architecture` section with:

```markdown
Single-binary Rust CLI. All logic lives in `src/main.rs`:

- **`Args`** — clap derive struct; positional `mirrors: Vec<String>`, `--probe-path` (default `/ubuntu/dists/noble/Release`), `--timeout` (default 3s), `--fast-threshold` (default 500ms), `--fast-count` (default 3), `--cache-file` (default `.selected-mirror.json`), `--no-cache`
- **`CacheEntry`** — serde struct persisted to JSON: `version`, `mirror`, `elapsed_ms`, `probe_path`, `recorded_at` (UNIX seconds)
- **`load_cache(path) -> Option<CacheEntry>`** — reads and deserializes the cache; returns `None` on missing file, bad JSON, or unrecognized version
- **`save_cache(path, mirror, elapsed_ms, probe_path)`** — serializes and writes atomically via `tmp + rename`; write failures print a warning and return without aborting
- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — fires a ureq GET, returns elapsed seconds or `None` on failure/timeout
- **`find_best(results: &[(String, Option<f64>)]) -> Option<&str>`** — pure fn; picks mirror with minimum elapsed time, `None` if all failed
- **`main`** — (1) if cache is valid and cached mirror is in argv, probe it; if elapsed < `--fast-threshold`, print and return; (2) otherwise spawn one `std::thread` per mirror, collect via `mpsc::channel`, print timing to stderr and winner to stdout; (3) write winner to cache; exits 1 if all mirrors fail

Integration tests in `tests/cli.rs` use `assert_cmd` and a raw `TcpListener`-based mock HTTP server (no external mock library). All tests pass `--cache-file` to an isolated `tempfile::TempDir` path.
```

- [ ] **Step 4: Verify the full test suite still passes**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add .gitignore README.md CLAUDE.md
git commit -m "docs: document cache flags; add .selected-mirror.json to .gitignore"
```
