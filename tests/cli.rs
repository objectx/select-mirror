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

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Second run: should get a cache hit
    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .args(["--cache-file", &cf])
        .args(["--fast-threshold", "5000"])
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
    // Verify the cache file was refreshed (recorded_at updated)
    let cache_content = std::fs::read_to_string(&cf).unwrap();
    let entry: serde_json::Value = serde_json::from_str(&cache_content)
        .expect("cache file should be valid JSON after cache hit");
    assert!(
        entry["recorded_at"].as_u64().unwrap_or(0) >= before,
        "cache hit should update recorded_at, got: {cache_content}"
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
    assert!(
        stderr.contains("malformed"),
        "expected 'malformed' warning in stderr for bad cache file, got: {stderr}"
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
    assert!(
        stderr.contains("unsupported version"),
        "expected 'unsupported version' warning in stderr, got: {stderr}"
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
fn cache_miss_when_cached_mirror_unreachable() {
    let port = start_mock_server();
    let live_mirror = format!("http://127.0.0.1:{}", port);
    let dead_mirror = "http://127.0.0.1:1".to_string(); // guaranteed-closed port
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Seed cache pointing to the dead mirror
    std::fs::write(
        &cf,
        format!(
            r#"{{"version":1,"mirror":"{}","elapsed_ms":10,"probe_path":"/","recorded_at":0}}"#,
            dead_mirror
        )
        .as_bytes(),
    )
    .unwrap();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&dead_mirror, &live_mirror])
        .args(["--probe-path", "/"])
        .args(["--timeout", "1"])
        .args(["--cache-file", &cf])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        live_mirror.as_str(),
        "live mirror should win after dead cached mirror falls through"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("unreachable"),
        "expected 'unreachable' in stderr when cached mirror is unreachable, got: {stderr}"
    );
    assert!(
        !stderr.contains("(cached)"),
        "expected no cache hit for unreachable mirror, got: {stderr}"
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

    // Record time before the run so we can verify recorded_at was refreshed
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

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

    // Cache file should have been rewritten with a fresh recorded_at (not the seeded 0)
    let cache_content = std::fs::read_to_string(&cf).unwrap();
    let entry: serde_json::Value = serde_json::from_str(&cache_content)
        .expect("cache file should be valid JSON after --no-cache run");
    assert!(
        entry["recorded_at"].as_u64().unwrap_or(0) >= before,
        "cache file should have a fresh recorded_at after --no-cache run, got: {cache_content}"
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
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("warning"),
        "expected a write-failure warning in stderr, got: {stderr}"
    );
}

#[test]
fn concurrent_cache_writes_produce_valid_json() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);
    let dir = TempDir::new().unwrap();
    let cf = cache_file(&dir);

    // Spawn 4 select-mirror processes against the same cache file simultaneously
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let mirror = mirror.clone();
            let cf = cf.clone();
            std::thread::spawn(move || {
                std::process::Command::new(
                    assert_cmd::cargo::cargo_bin("select-mirror"),
                )
                .args([&mirror])
                .args(["--probe-path", "/"])
                .args(["--cache-file", &cf])
                .output()
                .expect("select-mirror should run")
            })
        })
        .collect();

    for handle in handles {
        let output = handle.join().expect("thread should not panic");
        assert!(output.status.success(), "each invocation should succeed");
    }

    // Cache file must exist and be valid JSON after concurrent writes
    assert!(std::path::Path::new(&cf).exists(), "cache file should exist");
    let content = std::fs::read_to_string(&cf).unwrap();
    let _: serde_json::Value =
        serde_json::from_str(&content).expect("cache file should be valid JSON after concurrent writes");
}
