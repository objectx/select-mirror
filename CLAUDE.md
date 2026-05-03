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

- **`Args`** — clap derive struct; positional `mirrors: Vec<String>`, `--probe-path` (default `/ubuntu/dists/noble/Release`), `--timeout` (default 3s), `--fast-threshold` (default 500ms), `--fast-count` (default 3), `--cache-file` (default `.selected-mirror.json`), `--no-cache`
- **`CacheEntry`** — serde struct persisted to JSON: `version`, `mirror`, `elapsed_ms`, `probe_path`, `recorded_at` (UNIX seconds)
- **`CacheEntry::new(mirror, elapsed_ms, probe_path) -> Self`** — builds entry with current UNIX timestamp
- **`load_cache(path) -> Option<CacheEntry>`** — reads and deserializes the cache; returns `None` on missing file; warns and returns `None` on malformed JSON or unrecognized version
- **`save_cache(path, entry: &CacheEntry)`** — serializes and writes atomically via `tmp + rename`; write failures print a warning and return without aborting
- **`secs_to_ms(secs: f64) -> u64`** — converts elapsed seconds to integer milliseconds
- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — fires a ureq GET, returns elapsed seconds or `None` on failure/timeout
- **`find_best(results: &[(String, Option<f64>)]) -> Option<&str>`** — pure fn; picks mirror with minimum elapsed time, `None` if all failed
- **`main`** — (1) if cache is valid and cached mirror is in argv, probe it; if elapsed < `--fast-threshold`, print and return; (2) otherwise spawn one `std::thread` per mirror, collect via `mpsc::channel`, print timing to stderr and winner to stdout; (3) write winner to cache; exits 1 if all mirrors fail

Integration tests in `tests/cli.rs` use `assert_cmd` and a raw `TcpListener`-based mock HTTP server (no external mock library). All tests pass `--cache-file` to an isolated `tempfile::TempDir` path.

The original shell reference implementation is in `reference/select-mirror.sh`.
