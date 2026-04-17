# Early-Exit Mirror Selection Design

**Date:** 2026-04-17

## Problem

The current implementation probes all supplied mirrors before selecting the fastest. When many mirrors are provided, this wastes time waiting on slow or distant servers after a sufficient number of fast ones have already responded.

## Goal

Stop probing once a configurable number of mirrors have responded within a configurable latency threshold. Pick the fastest among the mirrors collected so far.

## CLI Interface

Two new flags added to `Args`:

```
--fast-threshold <ms>   Response time (ms) considered "fast" (default: 500)
--fast-count <n>        Stop after this many fast mirrors respond (default: 3)
```

Threshold is specified in milliseconds (integer) for shell friendliness. Internally converted to `f64` seconds for comparison against `probe()`'s elapsed return value.

## Architecture

No structural changes. The thread-per-mirror model and `mpsc::channel` remain unchanged.

### Receive loop change

A `fast_count_seen: usize` counter is added to the receive loop in `main`. After each result arrives:

1. If `elapsed < threshold`, increment `fast_count_seen`.
2. If `fast_count_seen >= args.fast_count`, `break` out of the loop.

Dropping `rx` causes remaining in-flight thread sends to return `Err`, which is already silently discarded via `let _ = tx.send(...)`. Background threads complete naturally, bounded by `--timeout`.

### Fallback behavior

If fewer than `--fast-count` mirrors ever respond under the threshold (all slow or all failing), the loop runs to completion — identical to current behavior. No regressions.

### `find_best` unchanged

Selects the minimum elapsed time from whatever results were collected before the loop exited.

## Testing

Two new integration tests in `tests/cli.rs`:

1. **Early exit succeeds**: Start 5 mock servers, pass all with `--fast-count 2 --fast-threshold 500`. Assert exit 0 and winner is one of the 5 mirrors.

2. **Fast-count exceeds mirror count**: Pass `--fast-count` larger than the number of mirrors. Assert all mirrors are evaluated and the best is still returned correctly (no crash, no hang).

Existing tests cover: exit-nonzero on all failures, correct winner selection — no changes needed there.

## Trade-offs

- Background threads linger until their `--timeout` fires after early exit. At default 3s timeout and 500ms threshold, stragglers die within 3s — acceptable for a CLI tool.
- True cancellation (async/atomic flag) would eliminate lingering threads but adds significant complexity for negligible practical benefit.
