## Context

`select-mirror` is a single-binary Rust CLI (`src/main.rs`) that probes a list of HTTP mirrors in parallel and prints the fastest one to stdout. It already supports an early-exit optimization (`--fast-count` mirrors qualifying within `--fast-threshold` ms ends the race). All logic lives in one file: `Args`, `probe`, `find_best`, `parse_fast_count`, `main`.

The output of this tool is consumed by Docker builds that pin the apt mirror into an image layer. Because today's selection is always the live race winner, network jitter alone produces different output across runs, invalidating the layer cache and forcing full rebuilds. The motivating constraint is therefore **stable output for stable inputs**, not fastest possible mirror per run.

This change adds a JSON cache file that records the most recent winner. On invocation, the tool probes the cached mirror first and short-circuits if it is still fast enough by the user's existing definition (`--fast-threshold`). On miss, slow probe, or failed probe, the existing parallel probe-all flow runs unchanged.

## Goals / Non-Goals

**Goals:**
- Stable output across consecutive invocations when the network has not materially changed.
- Reuse existing knobs (`--fast-threshold`) to define "fast enough"; do not introduce parallel concepts.
- Keep the single-file architecture and small dependency footprint.
- Preserve the existing probe-all behavior as a fallback path; cache is purely additive.
- Cache is per-project (CWD-relative default), gitignored, ephemeral.

**Non-Goals:**
- TTL-based cache expiry (deferred until someone asks).
- Adaptive thresholds based on historical timings.
- Cross-host or shared cache.
- Capturing the full mirror set fingerprint in the cache (overkill for this use case).
- Walking parent directories to find a project-root cache (explicit `--cache-file` is the escape hatch).

## Decisions

### Decision 1: Cache file in CWD by default

**Choice**: `.selected-mirror.json` in the current working directory; `--cache-file <path>` to override.

**Why**: The motivating use case is Docker builds that produce the same image layer when the selected mirror is unchanged. A CWD-relative default places the cache file inside the project, where it can be observed, deleted, or fed deliberately. An XDG-style user cache (`~/.cache/select-mirror/`) would be invisible to Docker contexts and would silently couple unrelated projects through a shared file.

**Alternatives considered**:
- XDG cache dir — rejected: defeats the per-project Docker-layer use case.
- Walking up to find a project root — rejected: implicit; the override flag handles legitimate variants.

### Decision 2: Strict argv-set membership

**Choice**: If the cached mirror is not present in the current `mirrors` argv, treat the cache as a miss.

**Why**: Passing a different mirror list is an explicit user signal. Honoring a removed mirror just because it is in the cache would surprise users. The override flag (`--cache-file`) is sufficient for cases where the user truly wants a stable file across changing argvs.

**Alternatives considered**:
- Lenient (try cached anyway if reachable) — rejected: violates user intent.

### Decision 3: Reuse `--fast-threshold` for cache validation

**Choice**: A cache hit qualifies if `cached-mirror probe < --fast-threshold`. No separate `--cache-threshold`.

**Why**: One knob, one definition of "fast enough." A separate threshold would force every user to reason about the difference. The semantics already match: `--fast-threshold` answers "is this mirror good enough to stop looking?" — exactly the cache-hit gate.

**Alternatives considered**:
- Separate `--cache-threshold` — rejected: knob proliferation, no real differentiation.
- Multiplier on the cached `elapsed_ms` (e.g., 2× baseline) — rejected: clever but harder to reason about; defer until evidence demands it.

### Decision 4: Cache miss path includes the cached mirror in the parallel probe

**Choice**: When falling through from a slow/failed cache probe, the cached mirror joins the parallel probe-all set with no special-casing or deduplication.

**Why**: Keeps the miss path identical to today's code. Re-probing the cached mirror once is a few hundred milliseconds at worst; deduplicating would add branching and state for no user-visible benefit.

**Alternatives considered**:
- Skip the cached mirror in the miss-path probe — rejected: optimization without payoff, complicates flow.

### Decision 5: Atomic cache write (tmp + rename)

**Choice**: Write to `<cache-file>.tmp.<pid>`, fsync (best-effort), then `rename` to `<cache-file>`.

**Why**: Concurrent invocations (parallel CI matrix in the same dir) must not produce a half-written file that fails to parse on the next run. `rename` is atomic on POSIX within the same filesystem.

**Alternatives considered**:
- Plain truncate-then-write — rejected: a crash mid-write leaves a corrupt cache.
- File locking — rejected: overkill for this risk profile.

### Decision 6: `--no-cache` is read-disable, still write

**Choice**: `--no-cache` bypasses cache reading on this run but still writes the fresh winner to the cache file.

**Why**: The intent of `--no-cache` is "force a re-probe right now" (network changed, debugging, etc.). Capturing the new winner is a feature, not a bug — it means subsequent runs benefit. Users who want full disable can pass `--cache-file /dev/null`.

**Alternatives considered**:
- `--no-cache` disables both read and write — rejected: forces users into `--cache-file /dev/null` for the common case of "re-probe but remember."

### Decision 7: Cache hit prints one stderr line

**Choice**: On cache hit, stderr emits `<url>: 0.312s (cached)` (or similar).

**Why**: Today the tool announces every probe on stderr. Silent cache hits would be confusing — users would not understand why nothing was probed. One line keeps the behavior observable while preserving the value of the short-circuit.

### Decision 8: Cache write failures are non-fatal

**Choice**: If writing the cache file fails (permissions, disk, EROFS), log a warning to stderr and exit 0 with the chosen mirror still on stdout.

**Why**: The user's primary signal is the chosen mirror. Failing the whole run because we cannot persist would be worse than printing the winner. Read-only filesystems and sandboxed CI runners are real environments.

**Alternatives considered**:
- Hard-fail on write error — rejected: punishes the primary use case for an ancillary failure.

### Decision 9: Add `serde` + `serde_json`

**Choice**: Pull in `serde` (with `derive`) and `serde_json` for cache serialization.

**Why**: Hand-rolling a 5-field JSON parser is defensible for binary size, but the cost in test surface and maintenance outweighs the few KB. The release profile (`lto`, `opt-level = "z"`, `strip`, `panic = "abort"`) keeps the impact small.

**Alternatives considered**:
- Hand-rolled JSON — rejected: maintenance burden, parser edge cases.
- TOML/YAML — rejected: heavier and less natural for this shape.

## Risks / Trade-offs

- **Cache hides a permanently slow mirror** → mitigated by re-validation on every run: a slow probe demotes the cache and falls through to probe-all, which then writes a new winner.
- **Default-on caching is a behavior change** → documented in `proposal.md` as breaking-by-behavior. `--no-cache` restores prior semantics. The tool is at v0.1.x; impact is bounded.
- **Network-changed scenario where cached probe still beats threshold** (e.g., user moved offices, but cached mirror is geographically incidentally near both) → acceptable: the threshold IS the user's definition of "good enough." If they want stricter, they tighten `--fast-threshold`.
- **Concurrent cache writes within the same filesystem** → addressed by atomic rename. Across filesystems (rare for a CWD-relative file), `rename` is also atomic on POSIX, but write fails if `tmp` and target are on different mounts; warning-and-continue covers it.
- **Cache file leaks into committed repos if `.gitignore` is forgotten** → mitigated by adding the entry as part of this change. Users who use `--cache-file` to a custom path are responsible for their own ignore rules.
- **Tests writing to CWD** → all new and existing tests must use a tempdir-isolated `--cache-file` so test runs do not pollute the workspace.

## Migration Plan

This is a behavior-only change to a v0.1.x CLI; there is no data migration. Users on the prior version see the new default-on cache behavior on upgrade. Anyone needing the old semantics passes `--no-cache`. No version-bump policy decisions required for this change beyond the version bump itself (deferred to release time).

Rollback: revert the commit; the cache file is forward-compatible with the prior version (which simply ignores it).

## Open Questions

None at this point — all four design questions resolved during exploration:
1. Cache file is gitignored.
2. Cached mirror not in argv is ignored.
3. `--fast-threshold` is reused.
4. No TTL until evidence demands one.
