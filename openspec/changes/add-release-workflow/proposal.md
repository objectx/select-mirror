## Why

`select-mirror` has no release automation: no GitHub Actions workflows,
no published binaries, no tags. Users must `cargo install` from source,
which requires a Rust toolchain on every target machine — friction for
the CLI's likely audience (people bootstrapping Ubuntu boxes who haven't
installed anything yet). Adding a tag-driven release pipeline now also
unlocks routine versioning practice on this repo before more changes
accumulate untagged on `main`.

## What Changes

**Release distribution**
- From: no published artifacts; users build from source.
- To: `v*` tag pushes produce a GitHub Release with three platform
  binaries (macOS arm64, Linux x86_64-gnu, Windows x86_64-msvc) and
  per-binary `.sha256` sidecars.
- Reason: lowers install friction; gives users verifiable downloads.
- Impact: non-breaking; new automation only.

**Pre-publish verification**
- From: no automated test gate before binaries reach users.
- To: every shipped binary passes `cargo test --release --target <target>`
  on its native runner before upload.
- Reason: release-profile (LTO + `panic = "abort"`) regressions are not
  caught by debug-mode tests; integration suite is offline-safe so it
  costs nothing to run on every release builder.
- Impact: non-breaking.

**Pipeline testability**
- From: pipelines could only be exercised by cutting a real tag.
- To: `workflow_dispatch` triggers the same matrix and uploads results
  to the run page (`actions/upload-artifact@v4`), without creating a
  GitHub Release.
- Reason: a dispatch run validates matrix correctness, checksum
  generation, and artifact naming before any user-visible Release is cut.
- Impact: non-breaking.

## Capabilities

### New Capabilities
- `release-build`: tag-triggered and dispatch-triggered automation that
  builds, tests, checksums, and publishes platform binaries — including
  the matrix shape, artifact naming convention, sha256 sidecar format,
  trigger semantics, and the `create-release` / `build` job gating that
  lets dispatch runs reuse the build job without producing a Release.

### Modified Capabilities

None. Existing capabilities (`mirror-selection-cache`, `aware-sandbox`)
are about runtime behavior; this change is about distribution.

## Impact

- **New file**: `.github/workflows/release.yml` (the entire workflow).
- **No code changes** to `src/`, `tests/`, `Cargo.toml`, or `Cargo.lock`.
  The release profile is already configured.
- **No new dependencies**. The workflow uses the GitHub-hosted runner
  toolchain, `actions/checkout@v4`, `actions/upload-artifact@v4`, and
  the `gh` CLI (preinstalled on all GitHub-hosted runners).
- **No secrets required**. `GITHUB_TOKEN` is automatically provisioned;
  no Apple Developer ID, code-signing cert, or PyPI/crates.io tokens
  needed.
- **First tag (`v0.1.0`)** can be cut after a successful dispatch dry
  run validates the matrix end-to-end.
