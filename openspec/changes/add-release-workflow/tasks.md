## 1. Workflow scaffolding

- [x] 1.1 Create the `.github/workflows/` directory at the repository root.
- [x] 1.2 Add `.github/workflows/release.yml` with workflow name `Release` and the two triggers: `push.tags: ['v*']` and `workflow_dispatch:`.

## 2. `create-release` job

- [x] 2.1 Add the `create-release` job: `runs-on: ubuntu-latest`, `permissions: { contents: write }`, gated with `if: github.event_name == 'push'`.
- [x] 2.2 Add a single step that runs `gh release create "${{ github.ref_name }}" --repo "${{ github.repository }}" --generate-notes` with `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`.

## 3. `build` job skeleton

- [x] 3.1 Add the `build` job with `name: Build (${{ matrix.os }}, ${{ matrix.target }})`, `needs: [create-release]`, `permissions: { contents: write }`, `defaults: { run: { shell: bash } }`, and `runs-on: ${{ matrix.os }}`.
- [x] 3.2 Set the gate `if: always() && (needs.create-release.result == 'success' || needs.create-release.result == 'skipped')` and add an inline YAML comment explaining the dispatch path requires the `skipped` branch.
- [x] 3.3 Configure `strategy.fail-fast: false` and `strategy.matrix.include` with three entries:
  - `os: macos-latest`, `target: aarch64-apple-darwin`, `bin_suffix: ""`, `checksum_cmd: "shasum -a 256"`
  - `os: ubuntu-24.04`, `target: x86_64-unknown-linux-gnu`, `bin_suffix: ""`, `checksum_cmd: "sha256sum"`
  - `os: windows-2025`, `target: x86_64-pc-windows-msvc`, `bin_suffix: ".exe"`, `checksum_cmd: "sha256sum"`

## 4. `build` job steps

- [x] 4.1 Add `actions/checkout@v4` step.
- [x] 4.2 Add `Add Rust target` step running `rustup target add ${{ matrix.target }}`.
- [x] 4.3 Add `Build` step running `cargo build --release --target ${{ matrix.target }}`.
- [x] 4.4 Add `Test` step running `cargo test --release --target ${{ matrix.target }}` (no `if:` — runs on every entry).
- [x] 4.5 Add `Rename binary` step copying `target/${{ matrix.target }}/release/select-mirror${{ matrix.bin_suffix }}` to `select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}` in the working directory.
- [x] 4.6 Add `Generate sha256` step running `${{ matrix.checksum_cmd }} <renamed_binary> > <renamed_binary>.sha256` from the working directory (so the embedded filename has no path prefix).

## 5. Conditional upload steps

- [x] 5.1 Add `Upload to GitHub Release` step gated by `if: github.event_name == 'push'`, running `gh release upload "${{ github.ref_name }}" "<binary>" "<binary>.sha256" --repo "${{ github.repository }}"` with `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`.
- [x] 5.2 Add `Upload as workflow artifact` step gated by `if: github.event_name == 'workflow_dispatch'`, using `actions/upload-artifact@v4` with `name: select-mirror-${{ matrix.target }}` and a `path:` listing both binary and sidecar.

## 6. Local validation

- [x] 6.1 Run `openspec validate add-release-workflow --strict` and resolve any reported issues.
- [x] 6.2 Run `actionlint .github/workflows/release.yml` if available locally; otherwise rely on push-time validation.

## 7. End-to-end verification on the actual repo

- [x] 7.1 Push the workflow file to a feature branch.
- [x] 7.2 Trigger `workflow_dispatch` (after merging to main; GitHub requires the workflow on the default branch to enable dispatch) and confirm: all three matrix entries pass; the run page lists three artifacts (`select-mirror-aarch64-apple-darwin`, `select-mirror-x86_64-unknown-linux-gnu`, `select-mirror-x86_64-pc-windows-msvc`); each artifact contains both a binary and a `.sha256` sidecar.
- [x] 7.3 Download each artifact locally and run `shasum -c <bin>.sha256` (macOS) or `sha256sum -c <bin>.sha256` (Linux/Windows) to confirm sidecar verification works.
- [x] 7.4 Confirm no GitHub Release was created by the dispatch run.
- [ ] 7.5 Push tag `v0.1.0` and verify the GitHub Release is created with auto-generated notes and all six expected files (3 binaries + 3 sidecars).
