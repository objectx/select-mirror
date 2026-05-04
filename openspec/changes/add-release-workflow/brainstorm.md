## Design Summary

Add a single GitHub Actions workflow at `.github/workflows/release.yml` that
publishes platform binaries with checksums when a `v*` tag is pushed, and
provides a `workflow_dispatch` path for end-to-end pipeline testing without
producing a real release.

The workflow consolidates three native-target builds into one matrix job
(macOS arm64, Linux x86_64, Windows x86_64), gated by a `create-release`
job that only runs on tag pushes. Every target is native to its runner, so
every target runs the integration test suite in release mode before its
artifact is published.

## Alternatives Considered

### Approach A: Direct port of `get-system-include-dirs/.github/workflows/release.yml`

- **Approach**: Copy the reference workflow verbatim, adjust binary name and
  drop the macOS matrix down to a single arm64 entry.
- **Pros**:
  - Minimum change surface — known-good template.
  - Three explicit jobs read clearly in the Actions UI.
  - No bash-on-Windows shell quirks to manage.
- **Cons**:
  - Three near-identical job blocks (~3× the YAML to maintain).
  - Reference smoke-tests only Windows; cross-platform regressions are not
    caught before upload despite `select-mirror` having a real integration suite.
  - No `workflow_dispatch` — pipeline can only be exercised by cutting a real tag.
  - No checksums — ironic for a tool whose job is downloading from mirrors.
- **Why not chosen**: Saves typing once, costs maintenance forever; ships
  unverified binaries on macOS and Linux.

### Approach B: Matrix-consolidated with dispatch + per-binary sha256 (chosen)

- **Approach**: One build job with three `include:` matrix entries
  (`os`, `target`, `checksum_cmd`, `bin_suffix`); `defaults: { run: { shell: bash } }`
  to unify Windows under Git Bash. `create-release` job runs only on push;
  build jobs run on both push and dispatch with explicit `if` gating that
  treats `skipped` as success. `cargo test --release --target <target>`
  runs on every entry (all native). Per-binary `<artifact>.sha256` sidecars
  using `shasum -a 256` on macOS and `sha256sum` elsewhere.
- **Pros**:
  - One source of truth for build/test/checksum logic.
  - Every shipped binary is integration-tested in release mode on its native runner.
  - `workflow_dispatch` exercises the full pipeline without polluting Releases.
  - Per-target `.sha256` files give users verifiable downloads.
- **Cons**:
  - Matrix entries must keep `os`, `target`, and `bin_suffix` in lockstep.
  - Bash-on-Windows is one more thing to remember when a step misbehaves.
  - `if: always() && (... == 'success' || == 'skipped')` is non-obvious and
    must be commented to survive future edits.
- **Why chosen**: Three targets that are nearly identical *should* be one
  job. The dispatch + checksum additions are small and meaningfully improve
  testability and trust without adding heavyweight machinery.

### Approach C: Heavy release pipeline (caching, archives, aggregated SHA256SUMS, signing)

- **Approach**: Approach B plus `Swatinem/rust-cache@v2`, `.tar.gz`/`.zip`
  archive packaging, a separate aggregation job producing one
  `SHA256SUMS` file, and macOS notarization / Windows code signing.
- **Pros**:
  - Faster cold builds (LTO + codegen-units=1 release builds are slow).
  - Single `sha256sum -c SHA256SUMS` UX for users.
  - Signed binaries don't trigger Gatekeeper / SmartScreen warnings.
- **Cons**:
  - Aggregation job adds a new fan-in dependency and an extra ~30s of overhead.
  - Signing requires secrets management (Apple Developer ID, code-signing cert)
    that the project doesn't currently have.
  - More moving parts, more failure modes, more YAML to maintain.
- **Why not chosen**: This is a small CLI; releases are infrequent so cold-build
  speed doesn't justify cache complexity yet. Signing is real work that
  belongs in a separate change once there's demand. Aggregated `SHA256SUMS`
  was explicitly de-scoped — per-binary sidecars cover the same trust need
  with simpler job topology.

## Agreed Approach

**Approach B**. The workflow will live at `.github/workflows/release.yml`
and consist of two jobs:

1. **`create-release`** — runs only on `push` of `v*` tags; calls
   `gh release create ${{ github.ref_name }} --generate-notes`.
2. **`build`** — matrix over three OS/target pairs; needs `create-release`
   but uses `if: always() && (needs.create-release.result == 'success' || needs.create-release.result == 'skipped')`
   so it also runs on `workflow_dispatch`. Each entry: checkout → add target →
   build → test → rename binary → emit `.sha256` sidecar → conditional upload
   (release upload on push, workflow artifact on dispatch).

Three native targets:

| OS runner       | Target                       | bin_suffix | checksum_cmd       |
|-----------------|------------------------------|------------|--------------------|
| macos-latest    | aarch64-apple-darwin         | (empty)    | `shasum -a 256`    |
| ubuntu-24.04    | x86_64-unknown-linux-gnu     | (empty)    | `sha256sum`        |
| windows-2025    | x86_64-pc-windows-msvc       | `.exe`     | `sha256sum`        |

## Key Decisions

- **Drop `x86_64-apple-darwin`** from the originally requested target list.
  Apple Silicon shipped 2020; macOS x86 is aging out and would be the only
  cross-compiled, untested target. Better to ship three verified binaries
  than four with one we can't run integration tests against.
- **Tests run on all three targets** (no `run_tests` matrix flag), because
  every target is native after dropping x86_64-darwin.
- **`ubuntu-24.04` accepted as Linux glibc floor** (glibc 2.39).
  Users on older distros can build from source; matches reference choice.
- **`workflow_dispatch` does not create a Release.** It uses
  `actions/upload-artifact@v4` so dispatch runs are testable from the run
  page without producing user-visible Releases.
- **Per-binary `<artifact>.sha256` sidecars**, not an aggregated
  `SHA256SUMS`. Simpler topology; same verification capability.
- **Single matrix-consolidated build job**, not three separate jobs.
  Three near-identical jobs are a maintenance liability when one source of
  truth would suffice.
- **`defaults: { run: { shell: bash } }` at the build job level** so the
  Windows entry uses Git Bash and shares rename + checksum step bodies with
  the other entries.
- **`needs:` + `if: always()` gating must be commented in the YAML.** The
  "skipped is success" idiom is a known GitHub Actions footgun; future
  edits could trivially break the dispatch path.

## Open Questions

None blocking. Items deferred to future changes if demand emerges:

- Aggregated `SHA256SUMS` file (de-scoped here).
- Archive formats (`.tar.gz`, `.zip`) — bare binaries are fine for a single-file CLI.
- Cargo build caching — release builds are infrequent.
- aarch64-linux, Windows ARM64 — not in current target list.
- Signing / notarization — separate change, requires secret provisioning.
- A separate CI workflow on PRs — orthogonal to this change.
