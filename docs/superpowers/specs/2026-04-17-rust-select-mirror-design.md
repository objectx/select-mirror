# select-mirror: Rust CLI Design

## Overview

A Rust reimplementation of `reference/select-mirror.sh` that probes a list of Ubuntu mirror
servers in parallel and prints the fastest one to stdout.

## CLI Interface

```
select-mirror [OPTIONS] <MIRROR>...

Arguments:
  <MIRROR>...  One or more mirror base URLs (e.g. http://ftp.jaist.ac.jp/pub/Linux/ubuntu)

Options:
  --probe-path <PATH>   Path appended to each mirror URL
                        [default: /ubuntu/dists/noble/Release]
  --timeout <SECS>      Request timeout in seconds [default: 3]
  -h, --help            Print help
```

## Architecture

Single binary crate. All logic in `src/main.rs`.

### Components

1. **CLI parsing** — `clap` (derive feature) parses positional mirror URLs and optional flags.
2. **Probe threads** — one `std::thread` per mirror; each measures wall-clock elapsed time
   for a GET request to `{mirror}{probe_path}` using `ureq` with the configured timeout.
3. **Result collection** — threads send `(mirror_url, Result<f64>)` over `std::sync::mpsc::channel`.
   Main thread collects exactly N results, then picks the minimum elapsed time.
4. **Output** — timing lines (`  {mirror}: {elapsed}s`) printed to stderr; winning mirror URL
   printed to stdout (same behavior as the shell script).

### Data Flow

```
CLI args
  └─> (mirror_url, probe_path, timeout) × N
        └─> thread per mirror
              └─> ureq GET → elapsed time
                    └─> mpsc channel
                          └─> collect N results
                                └─> pick min elapsed → stdout
                                └─> timing lines     → stderr
```

## Error Handling

- Per-mirror errors (timeout, network failure, non-2xx response) are treated as failed probes
  and excluded from the winner selection.
- If **all** mirrors fail, print an error message to stderr and exit with code 1.
- At least one successful probe → exit code 0.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (derive feature) |
| `ureq` | Lightweight synchronous HTTP client |

## Out of Scope

- Async runtime (tokio, async-std)
- Output formats other than plain URL
- Caching results
