# Implementation Milestones

## Milestones

- [x] **Milestone 1: Cache schema and I/O**
  Define the JSON cache schema, add serde dependencies, and implement loading (with version- and shape-gated validation that treats malformed files as misses) and atomic writing (`tmp + rename`). Satisfies the cache-file-schema, validation-gate, and atomic-write requirements.

- [x] **Milestone 2: Cache-hit short-circuit**
  Implement the single-probe-of-cached-mirror flow with the existing `--fast-threshold` as the gate, including the `<url>: <elapsed>s (cached)` stderr line. A valid, fast cached entry skips the probe-all flow and is printed to stdout unchanged. Satisfies the cache-hit-short-circuit and stderr-observability requirements.

- [x] **Milestone 3: Fall-through and flag plumbing**
  Wire the cache validation gate into `main`, add `--cache-file` and `--no-cache` flags, ensure miss/slow/failed cache paths fall through to the unchanged probe-all flow, and ensure cache writes happen on every successful selection (including under `--no-cache`). Cache write failures emit a stderr warning and exit 0. Satisfies the cache-miss-fall-through, cache-write-on-success, no-cache-flag, write-failures-non-fatal, and all-mirrors-failed exit-semantics requirements.

- [x] **Milestone 4: Test coverage**
  Extend the integration-test suite to exercise: cache hit, miss-by-each-validation-condition, slow-cached falls through, `--no-cache` skips read but still writes, malformed cache file, version mismatch, atomic write under concurrent invocations, write-failure non-fatal behavior, and stable-output across two consecutive invocations. All tests use a tempdir-isolated cache path so they do not pollute CWD.

- [x] **Milestone 5: Repo hygiene and docs**
  Add `.selected-mirror.json` to `.gitignore`, update `README` with the new flags and default-on caching behavior, and update `CLAUDE.md`'s architecture summary so the cache-aware control flow is documented for future contributors.
