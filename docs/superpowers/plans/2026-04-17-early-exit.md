# Early-Exit Mirror Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop probing mirrors once a configurable number have responded within a configurable latency threshold, then return the fastest of those collected.

**Architecture:** Two new CLI flags (`--fast-threshold`, `--fast-count`) are added to `Args`. The receive loop in `main` counts fast responses and breaks early once the count is met. No changes to `probe()` or `find_best()`.

**Tech Stack:** Rust, clap (derive), std::sync::mpsc, assert_cmd (integration tests)

---

## Files

- Modify: `src/main.rs` — add two `Args` fields; update receive loop
- Modify: `tests/cli.rs` — add two new integration tests

---

### Task 1: Write failing integration tests

**Files:**
- Modify: `tests/cli.rs`

- [ ] **Step 1: Add two new tests to `tests/cli.rs`**

Append after the last test in the file:

```rust
#[test]
fn early_exit_when_fast_count_met() {
    let ports: Vec<u16> = (0..5).map(|_| start_mock_server()).collect();
    let mirrors: Vec<String> = ports
        .iter()
        .map(|p| format!("http://127.0.0.1:{}", p))
        .collect();

    let mut cmd = Command::cargo_bin("select-mirror").unwrap();
    cmd.args(&mirrors)
        .args(["--probe-path", "/"])
        .args(["--fast-count", "2"])
        .args(["--fast-threshold", "500"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        mirrors.iter().any(|m| m == winner),
        "winner {winner} not in mirror list"
    );
}

#[test]
fn fast_count_exceeds_mirror_count_still_succeeds() {
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .args(["--fast-count", "10"])
        .args(["--fast-threshold", "500"])
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

- [ ] **Step 2: Run the new tests to confirm they fail**

```bash
cargo test --test cli early_exit
```

Expected: compilation succeeds but both tests fail — `select-mirror` exits nonzero because `--fast-count` and `--fast-threshold` are unrecognized flags.

---

### Task 2: Add `--fast-threshold` and `--fast-count` flags to `Args`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add two fields to the `Args` struct**

In `src/main.rs`, replace the `Args` struct with:

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

    /// Response time threshold in milliseconds considered "fast"
    #[arg(long, default_value_t = 500u64)]
    fast_threshold: u64,

    /// Stop after this many mirrors respond within --fast-threshold
    #[arg(long, default_value_t = 3usize)]
    fast_count: usize,
}
```

- [ ] **Step 2: Verify the project compiles**

```bash
cargo build
```

Expected: compiles with no errors. The new flags are wired to clap but not yet used in logic.

- [ ] **Step 3: Re-run the new tests to check they now compile and progress**

```bash
cargo test --test cli early_exit
```

Expected: tests may pass or still fail depending on default behavior — that's fine. We care only that `--fast-count` and `--fast-threshold` are now accepted without error.

---

### Task 3: Implement early-exit logic in the receive loop

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update the receive loop in `main`**

Replace the entire `main` function body in `src/main.rs` with:

```rust
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
        if elapsed.map_or(false, |e| e < threshold_secs) {
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
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass, including the two new integration tests and all existing unit/integration tests.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs tests/cli.rs
git commit -m "feat: add early-exit once fast-count mirrors meet fast-threshold"
```
