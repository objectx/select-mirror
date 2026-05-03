## Why

`select-mirror` re-probes every mirror on every invocation and picks whichever wins the race that run. Network jitter shuffles the leader, so the chosen mirror changes between otherwise-identical invocations. When the output is consumed by a Docker build (the primary use case), each new winner invalidates the apt-mirror layer and triggers a full image rebuild. Stable output for stable inputs is the goal — finding the fastest mirror is the means, not the end.

## What Changes

- Persist the most recent selection to a JSON cache file (default: `.selected-mirror.json` in CWD; override with `--cache-file <path>`).
- On startup, validate the cache and probe the cached mirror first. If it succeeds within `--fast-threshold`, output it immediately without probing the rest.
- On cache miss, slow probe, or failed probe of the cached mirror, fall through to the existing parallel probe-all flow unchanged.
- Always write the winning mirror back to the cache file (atomic `tmp + rename`), regardless of which path was taken.
- Add `--no-cache` flag: skip reading the cache, but still write the fresh result.
- Add `.selected-mirror.json` to `.gitignore`.
- **BREAKING (behavior)**: existing scripts that depend on always-fresh probing now get cache-first behavior by default. Pass `--no-cache` for the prior semantics.

## Capabilities

### New Capabilities
- `mirror-selection-cache`: Persist the selected mirror across invocations and short-circuit probing when the cached mirror is still fast enough.

### Modified Capabilities
<!-- None — there is no existing spec for the un-specced "fastest mirror selection" behavior; that contract remains as documented in src/main.rs. -->

## Impact

- **Code**: `src/main.rs` (cache load/validate/write, single-mirror fast-path branch, new flags), `Cargo.toml` (add `serde`, `serde_json`), `.gitignore` (add cache filename).
- **Tests**: `tests/cli.rs` extends to cover cache hit, cache miss, cache stale (probe-path mismatch, mirror absent from argv), cache corrupt, and `--no-cache`. Tests must use a tempdir-isolated `--cache-file` so they don't pollute CWD.
- **Dependencies**: adds `serde` + `serde_json` to dependencies. Acceptable cost for the JSON round-trip; release profile (`lto`, `opt-level = "z"`, `strip`, `panic = "abort"`) keeps binary size impact minimal.
- **Stderr output**: cache hits print one informational line (e.g., `<url>: 0.312s (cached)`) so the tool's behavior remains observable.
- **Consumers**: any consumer relying on the chosen mirror as a Docker layer cache key now gets stable output — the motivating use case. Consumers that explicitly want fresh selection use `--no-cache`.
- **No TTL** on cache entries in this change; cached selections age out only via probe failure or threshold violation. May revisit if needed.
