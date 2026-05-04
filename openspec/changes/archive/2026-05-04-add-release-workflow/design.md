## Context

`select-mirror` is a single-binary Rust CLI (`src/main.rs`) that probes Ubuntu
mirrors and selects the fastest. The repository currently has no release
automation: no GitHub Actions workflows, no tags, no published binaries.
Users must `cargo install` from source.

A sibling project (`get-system-include-dirs`, located at
`../get-system-include-dirs/.github/workflows/release.yml`) ships release
binaries via a tag-triggered Actions workflow. Its structure — one
`create-release` job followed by per-OS build jobs that upload artifacts —
is the starting template for this change. The reference targets four
platforms across three runners with a per-target rename + upload pattern,
smoke-tests Windows only, and ships bare binaries with no checksums.

`select-mirror` has stronger test affordances than the reference: a real
integration suite in `tests/cli.rs` (using `assert_cmd` and a hand-rolled
`TcpListener` mock server, no live network). That suite is fast and
self-contained — it can run on every release builder.

`Cargo.toml` already configures a tight release profile (`strip = true`,
`lto = true`, `opt-level = "z"`, `codegen-units = 1`, `panic = "abort"`).
`Cargo.lock` is committed for reproducible release builds.

Constraints:

- No GitHub Actions workflows currently exist; this change introduces the
  first one.
- The repository has no Apple Developer ID, code-signing certificate, or
  notarization secrets. Anything requiring secret provisioning is out of
  scope.
- macOS GitHub-hosted runners are arm64; running x86_64 macOS binaries
  natively is not possible.
- LTO + `codegen-units = 1` release builds are slow (multi-minute) per
  target. No build cache is being introduced in this change.

## Goals / Non-Goals

**Goals:**

- Publish verified release binaries for three platforms when a `v*` tag
  is pushed: macOS arm64, Linux x86_64 (gnu), Windows x86_64 (msvc).
- Run the integration test suite in release mode against every shipped
  binary on its native runner, before upload.
- Provide a `workflow_dispatch` trigger that exercises the full
  build/test/checksum pipeline without producing a public Release.
- Ship a `<artifact>.sha256` sidecar next to every binary so users can
  verify downloads with `shasum -c` / `sha256sum -c`.
- Keep the workflow as a single source of truth — three near-identical
  build jobs collapsed into one matrix.

**Non-Goals:**

- `x86_64-apple-darwin`. Removed from the original target list. Apple
  Silicon is the macOS reality; the only x86_64-darwin path would be
  cross-compile-on-arm64 with no native test coverage. Better to ship
  three verified binaries than four with one we can't run.
- Aggregated `SHA256SUMS` file. Per-binary sidecars are sufficient.
- Archive packaging (`.tar.gz`, `.zip`). Bare binaries are appropriate
  for a single-file CLI.
- Cargo build caching (`Swatinem/rust-cache@v2`). Releases are infrequent
  enough that cold-build time isn't worth the cache-management surface.
- aarch64 Linux, Windows ARM64. Not in current target list.
- Code signing / notarization. Requires secret provisioning; future change.
- A separate CI workflow on PRs. Orthogonal to this change.

## Decisions

### Two jobs: `create-release` (gate) + `build` (matrix)

The reference uses four jobs: `create-release` + three build jobs. With
all build jobs now near-identical, three of them collapse into one matrix
job. Final shape:

```
   push tag v*                  workflow_dispatch
        │                              │
        ▼                              │
  create-release ────(skip on dispatch)
        │                              │
        └──────────────┬───────────────┘
                       ▼ (build job, 3 matrix entries)
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   macos-latest   ubuntu-24.04   windows-2025
   aarch64-       x86_64-        x86_64-
    apple-         unknown-        pc-windows-
    darwin         linux-gnu       msvc

         each entry: build → test → sha256
                       │
              if push:     gh release upload  bin + bin.sha256
              if dispatch: actions/upload-artifact   bin + bin.sha256
```

**Why**: Three jobs that differ only in `os`, `target`, `bin_suffix`, and
`checksum_cmd` should be one matrix job with those four variables in
`include:`. Keeps build/test/checksum logic in one place; matrix entry
labels in the Actions UI remain readable (`Build (windows-2025,
x86_64-pc-windows-msvc)`).

**Alternative considered (separate jobs)**: Easier to debug per-platform
failures by job name; simpler `if` conditions because no matrix variables.
Rejected because the maintenance cost of three duplicated job blocks
outweighs the ergonomic gain.

### Matrix shape

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - os: macos-latest
        target: aarch64-apple-darwin
        bin_suffix: ""
        checksum_cmd: "shasum -a 256"
      - os: ubuntu-24.04
        target: x86_64-unknown-linux-gnu
        bin_suffix: ""
        checksum_cmd: "sha256sum"
      - os: windows-2025
        target: x86_64-pc-windows-msvc
        bin_suffix: ".exe"
        checksum_cmd: "sha256sum"
```

**`fail-fast: false`**: A failure on one platform should not cancel the
others. Partial release info (which target failed) is more useful than
fast termination.

**`shasum -a 256` on macOS, `sha256sum` elsewhere**: macOS does not ship
`sha256sum` by default. Both tools produce identical `<hash>  <filename>`
output, so downstream verification is uniform.

**Bash on Windows**: `defaults: { run: { shell: bash } }` at the build
job level. The `windows-2025` runner ships Git Bash, which provides
`sha256sum`, `cp`, and a Unix-shaped path layout — letting all three
matrix entries share the same step bodies. The only Windows-specific
input is `bin_suffix: ".exe"`, supplied via the matrix.

### `workflow_dispatch` does not produce a Release

The dispatch path's purpose is testability: validate that the matrix,
checksums, and binary names all produce expected outputs before cutting
a tag. Two gating rules:

1. **`create-release` skips on dispatch:**
   ```yaml
   create-release:
     if: github.event_name == 'push'
   ```
2. **`build` runs on both, with explicit success-or-skipped guard:**
   ```yaml
   build:
     needs: [create-release]
     if: always() && (needs.create-release.result == 'success'
                   || needs.create-release.result == 'skipped')
   ```

The `if: always() && (... == 'success' || == 'skipped')` idiom is
non-obvious — by default, when a `needs:` job is skipped, downstream jobs
also skip. The `always()` opens the gate, the explicit result check
keeps it from running when `create-release` actually fails. This **must**
ship with an inline comment in the YAML; future edits could trivially
break the dispatch path.

**Per-step upload conditional**:

```yaml
- name: Upload to GitHub Release
  if: github.event_name == 'push'
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: gh release upload "${{ github.ref_name }}" "<bin>" "<bin>.sha256" --repo "${{ github.repository }}"

- name: Upload as workflow artifact
  if: github.event_name == 'workflow_dispatch'
  uses: actions/upload-artifact@v4
  with:
    name: select-mirror-${{ matrix.target }}
    path: |
      <bin>
      <bin>.sha256
```

### Test in release mode on every matrix entry

After dropping `x86_64-apple-darwin`, every shipped target is native to
its runner. The build job runs `cargo test --release --target ${{ matrix.target }}`
unconditionally — no `run_tests` matrix flag needed. This catches release-
profile regressions (LTO/inlining-induced bugs, panic=abort path differences)
that a debug-mode CI run would miss.

The integration suite uses `assert_cmd` + a hand-rolled `TcpListener` mock
server with no external services, so it runs offline on any GitHub-hosted
runner.

### Artifact naming

```
select-mirror-aarch64-apple-darwin
select-mirror-aarch64-apple-darwin.sha256
select-mirror-x86_64-unknown-linux-gnu
select-mirror-x86_64-unknown-linux-gnu.sha256
select-mirror-x86_64-pc-windows-msvc.exe
select-mirror-x86_64-pc-windows-msvc.exe.sha256
```

Pattern: `<crate-name>-<target><bin_suffix>` for binaries, append `.sha256`
for sidecars. Matches the reference convention; scriptable; chmod-and-run.

The `.sha256` files contain a single line in standard sha256sum/shasum
format — `<hash>  <filename>` (text mode) or `<hash> *<filename>` (binary
mode, default for `sha256sum` on `.exe` under Git Bash) — with no path
prefix on the filename. The checksum step runs from the working directory
after rename, so the embedded filename is the bare artifact name. This
makes `sha256sum -c <bin>.sha256` (or `shasum -c`) work with the file in
the same directory, matching user expectations.

### Permissions

`create-release` and `build` (when running on push) both need
`contents: write` to call `gh release create` / `gh release upload`. The
dispatch path doesn't need it but inheriting `contents: write` is harmless
and avoids per-step permission scoping.

## Risks / Trade-offs

- **Matrix `if` gating is fragile** → Mitigated by inline YAML comment
  explaining the `success || skipped` pattern. A future edit that
  refactors the gate without preserving the semantic could silently
  break the dispatch path; reviewers should treat changes to that line
  as load-bearing.

- **Bash on Windows runners can produce surprising errors** for path
  handling, line endings, or quoting → Mitigated by keeping step bodies
  minimal and using `cp` + `sha256sum` (well-supported in Git Bash). If
  a future step needs PowerShell-specific behavior, it can opt out with
  `shell: pwsh` on that step.

- **Cold release builds are slow** (LTO + `codegen-units = 1` per target,
  ~3-5 minutes each, plus integration tests) → Accepted. Releases are
  infrequent. If this becomes a pain point, add `Swatinem/rust-cache@v2`
  in a follow-up.

- **glibc 2.39 floor on Linux binary** → Documented Non-Goal. Users on
  older distros build from source. If demand emerges, a future change
  can switch the Linux job to `ubuntu-22.04` (glibc 2.35) or add a
  `musl` static target.

- **`fail-fast: false` means partial publishes** are possible. If macOS
  succeeds but Windows fails on a real tag push, the Release will exist
  with two binaries → Mitigated by the dispatch path: testing the full
  matrix on dispatch before tagging surfaces these failures without
  needing a recovery path on the release path.

- **Action version pinning by major (`@v4`)** picks up minor/patch updates
  automatically. Could break on an upstream regression → Trade-off
  accepted; pinning to a SHA would be safer but adds maintenance overhead
  disproportionate to the project's release cadence.

- **No code signing** → macOS users will see Gatekeeper warnings on first
  run; Windows users will see SmartScreen warnings. Documented Non-Goal.
  Users can `xattr -d com.apple.quarantine` / "Run anyway" as a workaround.

## Migration Plan

This change introduces new automation; nothing existing is being replaced.

1. Land the workflow file in `main`.
2. Run `workflow_dispatch` from the Actions UI to verify all three matrix
   entries build, test, checksum, and upload artifacts cleanly.
3. Download artifacts from the dispatch run, verify file sizes and
   `sha256sum -c` succeed locally.
4. Cut the first tag (`v0.1.0`) to exercise the full release path and
   confirm the GitHub Release is created with all six files (3 binaries
   + 3 checksums) and auto-generated notes.

**Rollback**: Delete the workflow file. No state is created outside of
GitHub Releases (which can be deleted manually if a bad release is published).
The `create-release` job uses `--generate-notes`, so undoing a botched
release is `gh release delete <tag>` plus deleting the tag.
