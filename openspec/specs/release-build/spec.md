# release-build Specification

## Purpose
TBD - created by archiving change add-release-workflow. Update Purpose after archive.
## Requirements
### Requirement: Workflow location and triggers

A GitHub Actions workflow file SHALL exist at `.github/workflows/release.yml`.
The workflow MUST be triggered by exactly two events:

1. `push` of any tag matching the glob `v*`.
2. `workflow_dispatch` (manual trigger from the Actions UI), with no required inputs.

#### Scenario: Tag push triggers the workflow

- **WHEN** a maintainer pushes a tag matching `v*` (for example `v0.1.0`) to the repository
- **THEN** the workflow runs end-to-end, producing a GitHub Release with platform binaries and checksums

#### Scenario: Manual dispatch triggers the workflow

- **WHEN** a maintainer triggers the workflow manually via the Actions UI on any branch
- **THEN** the workflow runs the build matrix and uploads artifacts to the workflow run page without creating a GitHub Release

#### Scenario: Non-matching tag does not trigger the workflow

- **WHEN** a tag that does not match `v*` (for example `nightly` or `2026-05-04`) is pushed
- **THEN** the workflow does not run

### Requirement: Release creation gated to tag push

The workflow SHALL contain a `create-release` job that calls
`gh release create "${{ github.ref_name }}" --generate-notes` exactly when
the trigger event is `push`. The job MUST be skipped on `workflow_dispatch`
events via an `if: github.event_name == 'push'` guard.

#### Scenario: Tag push creates the GitHub Release

- **WHEN** the workflow runs because of a `v*` tag push
- **THEN** a GitHub Release named `${{ github.ref_name }}` exists with auto-generated release notes before any binary is uploaded

#### Scenario: Dispatch does not create a Release

- **WHEN** the workflow runs because of `workflow_dispatch`
- **THEN** the `create-release` job is skipped and no GitHub Release is created or modified

### Requirement: Build matrix targets

The workflow SHALL contain a `build` job whose `strategy.matrix.include`
defines exactly three entries, one per supported platform:

| `os`            | `target`                       | `bin_suffix` | `checksum_cmd`     |
|-----------------|--------------------------------|--------------|--------------------|
| `macos-latest`  | `aarch64-apple-darwin`         | (empty)      | `shasum -a 256`    |
| `ubuntu-24.04`  | `x86_64-unknown-linux-gnu`     | (empty)      | `sha256sum`        |
| `windows-2025`  | `x86_64-pc-windows-msvc`       | `.exe`       | `sha256sum`        |

The matrix MUST set `fail-fast: false` so that a failure on one entry does
not cancel the others.

#### Scenario: Three matrix entries run

- **WHEN** the workflow's `build` job starts
- **THEN** exactly three matrix entries are dispatched, one per OS/target pair listed above

#### Scenario: One platform failure does not cancel others

- **WHEN** one matrix entry fails (build, test, or upload)
- **THEN** the remaining matrix entries continue running to completion

### Requirement: Build job gating across both events

The `build` job MUST declare `needs: [create-release]` and gate its
execution with the explicit guard:

```yaml
if: always() && (needs.create-release.result == 'success'
              || needs.create-release.result == 'skipped')
```

This SHALL allow the build job to run on both event types: when
`create-release` succeeded (tag push) and when it was skipped
(dispatch). The build job MUST NOT run when `create-release` actually
fails or is cancelled.

#### Scenario: Build runs when create-release succeeds

- **WHEN** the workflow runs on a `push` event and the `create-release` job completes successfully
- **THEN** the `build` job runs all three matrix entries

#### Scenario: Build runs when create-release is skipped

- **WHEN** the workflow runs on a `workflow_dispatch` event and the `create-release` job is skipped by its `if` guard
- **THEN** the `build` job runs all three matrix entries

#### Scenario: Build does not run when create-release fails

- **WHEN** the workflow runs on a `push` event and the `create-release` job fails
- **THEN** the `build` job is not started

### Requirement: Pre-upload native test

Each matrix entry MUST run `cargo test --release --target ${{ matrix.target }}`
on its native runner before any artifact upload step. The entry SHALL
fail and skip upload if any test fails.

#### Scenario: Tests run in release mode on native targets

- **WHEN** the build job runs any matrix entry
- **THEN** the entry executes `cargo test --release --target <its target>` on its assigned `os` runner before any rename, checksum, or upload step

#### Scenario: Test failure aborts that entry's upload

- **WHEN** `cargo test` fails for one matrix entry
- **THEN** that entry does not produce a rename, sha256 sidecar, or upload step, and the entry is reported as failed

### Requirement: Artifact naming convention

After a successful build, each matrix entry MUST produce a renamed binary
and a sha256 sidecar in the workflow's working directory using these
patterns:

- Binary: `select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}`
- Sidecar: `select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}.sha256`

Concretely the six expected artifact names per release are:

```
select-mirror-aarch64-apple-darwin
select-mirror-aarch64-apple-darwin.sha256
select-mirror-x86_64-unknown-linux-gnu
select-mirror-x86_64-unknown-linux-gnu.sha256
select-mirror-x86_64-pc-windows-msvc.exe
select-mirror-x86_64-pc-windows-msvc.exe.sha256
```

#### Scenario: Linux artifacts use bare names

- **WHEN** the `ubuntu-24.04` matrix entry completes successfully
- **THEN** the working directory contains `select-mirror-x86_64-unknown-linux-gnu` and `select-mirror-x86_64-unknown-linux-gnu.sha256`

#### Scenario: Windows artifacts include `.exe` suffix

- **WHEN** the `windows-2025` matrix entry completes successfully
- **THEN** the working directory contains `select-mirror-x86_64-pc-windows-msvc.exe` and `select-mirror-x86_64-pc-windows-msvc.exe.sha256`

### Requirement: SHA-256 sidecar format

Each matrix entry MUST run `${{ matrix.checksum_cmd }} <renamed_binary>` from
the working directory after rename, redirecting output to
`<renamed_binary>.sha256`. The sidecar contents MUST be a single line in
the standard sha256sum/shasum output format — a 64-character lowercase hex
hash followed by the bare binary filename with no path prefix — using
either text-mode (`<hash>  <name>`) or binary-mode (`<hash> *<name>`)
representation, so that `shasum -c <sidecar>` and `sha256sum -c <sidecar>`
both succeed when run from the same directory as the binary.

#### Scenario: Sidecar contains hash and bare filename

- **WHEN** any matrix entry's checksum step completes
- **THEN** the sidecar file is exactly one line in standard sha256sum format containing a 64-character lowercase hex hash and the bare binary filename with no leading directory components, in either text-mode or binary-mode representation

#### Scenario: Sidecar verifies against its binary

- **WHEN** a user downloads a binary and its `.sha256` sidecar from a release into the same directory
- **THEN** running `shasum -c <binary>.sha256` (macOS) or `sha256sum -c <binary>.sha256` (Linux/Windows) succeeds

### Requirement: Conditional artifact upload by event

Each matrix entry's upload step MUST be selected by the trigger event:

- `if: github.event_name == 'push'` → upload via
  `gh release upload "${{ github.ref_name }}" "<binary>" "<sidecar>" --repo "${{ github.repository }}"`
  using `GITHUB_TOKEN` from `secrets.GITHUB_TOKEN`.
- `if: github.event_name == 'workflow_dispatch'` → upload via
  `actions/upload-artifact@v7`, naming the artifact
  `select-mirror-${{ matrix.target }}` and including both files.

Exactly one of these two upload paths MUST run per matrix entry per workflow invocation.

#### Scenario: Tag push uploads to the GitHub Release

- **WHEN** the workflow ran because of a `v*` tag push and a matrix entry's tests passed
- **THEN** that entry's binary and sidecar appear as assets on the GitHub Release named `${{ github.ref_name }}`

#### Scenario: Dispatch uploads as workflow artifact

- **WHEN** the workflow ran because of `workflow_dispatch` and a matrix entry's tests passed
- **THEN** that entry's binary and sidecar appear as a downloadable artifact on the workflow run page named `select-mirror-<target>`

#### Scenario: No double-publishing

- **WHEN** any matrix entry completes successfully under any trigger
- **THEN** the binary appears in exactly one location (Release OR run-page artifact), never both

### Requirement: Permissions and shell defaults

The `create-release` and `build` jobs MUST declare `permissions: { contents: write }`
so `gh release create` / `gh release upload` can write to the Release.
The `build` job MUST set `defaults: { run: { shell: bash } }` at job level
so that the Windows entry uses Git Bash (which provides `cp` and
`sha256sum`), allowing all matrix entries to share the same step bodies.

#### Scenario: Windows steps run under bash

- **WHEN** the `windows-2025` matrix entry executes its rename and checksum steps
- **THEN** the steps run under `bash` (Git Bash) and `sha256sum` is available without explicit installation

#### Scenario: Release upload has write permission

- **WHEN** any matrix entry runs the `gh release upload` step on a tag push
- **THEN** the call succeeds without `403 Forbidden` because the job has `contents: write` permission

### Requirement: Inline documentation of dispatch gating

The workflow file SHALL include an inline YAML comment immediately
adjacent to the `build` job's `if: always() && (... == 'success' || == 'skipped')`
guard, explaining that the `skipped` branch is required for the
`workflow_dispatch` path to work. The comment MUST reference the
dispatch event so a future maintainer reading only the workflow file
understands why both `success` and `skipped` are accepted. This
comment is part of the spec because the gating idiom is non-obvious
and trivially broken by future edits.

#### Scenario: YAML reviewer sees the rationale

- **WHEN** a reviewer reads the `build` job's `if:` line in the workflow file
- **THEN** an adjacent comment explains why both `success` and `skipped` are accepted, referencing the dispatch path

