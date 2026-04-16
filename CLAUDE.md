# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                  # debug build
cargo build --release        # optimized release build
cargo test                   # run all tests (unit + integration)
cargo test <name>            # run a single test by name
cargo test --test cli        # run only integration tests in tests/cli.rs
```

## Architecture

Single-binary Rust CLI. All logic lives in `src/main.rs`:

- **`Args`** — clap derive struct; positional `mirrors: Vec<String>`, `--probe-path` (default `/ubuntu/dists/noble/Release`), `--timeout` (default 3s)
- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — fires a ureq GET, returns elapsed seconds or `None` on failure/timeout
- **`find_best(results: &[(String, Option<f64>)]) -> Option<&str>`** — pure fn; picks mirror with minimum elapsed time, `None` if all failed
- **`main`** — spawns one `std::thread` per mirror, collects results via `mpsc::channel`, prints timing to stderr and the winner to stdout; exits 1 if all mirrors fail

Integration tests in `tests/cli.rs` use `assert_cmd` and a raw `TcpListener`-based mock HTTP server (no external mock library).

The original shell reference implementation is in `reference/select-mirror.sh`.
