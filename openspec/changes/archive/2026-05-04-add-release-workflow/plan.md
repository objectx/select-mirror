# Release Workflow Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development
> to implement this plan task-by-task.

**Goal:** Add `.github/workflows/release.yml` so that pushing a `v*` tag publishes platform binaries with sha256 sidecars, and `workflow_dispatch` exercises the same matrix without producing a Release.

**Architecture:** Single workflow file with two jobs — `create-release` (gated to `push` events, calls `gh release create --generate-notes`) and `build` (matrix of three native targets, gated with the `success || skipped` idiom so it runs on both events). Per-matrix-entry steps: checkout → add target → build → test → rename → sha256 → conditional upload (Release on push, run-page artifact on dispatch).

**Tech Stack:** GitHub Actions YAML, GitHub-hosted runners (`macos-latest`, `ubuntu-24.04`, `windows-2025`), `actions/checkout@v4`, `actions/upload-artifact@v4`, `gh` CLI, `cargo`, `rustup`, `shasum -a 256` (macOS), `sha256sum` (Linux + Windows Git Bash).

---

## Task 1: Workflow scaffolding

- [ ] **Step 1:** Create directory `.github/workflows/` at the repository root.
- [ ] **Step 2:** Create `.github/workflows/release.yml` with `name: Release` and the `on:` block declaring both triggers: `push.tags: ['v*']` and `workflow_dispatch:` (no inputs).

## Task 2: `create-release` job

- [ ] **Step 1:** Add the `create-release` job header with `runs-on: ubuntu-latest`, `permissions: { contents: write }`, and `if: github.event_name == 'push'` so it skips on dispatch.
- [ ] **Step 2:** Add the single step that calls `gh release create "${{ github.ref_name }}" --repo "${{ github.repository }}" --generate-notes`, supplying `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` via `env:`.

## Task 3: `build` job header and matrix

- [ ] **Step 1:** Add the `build` job header: `name: Build (${{ matrix.os }}, ${{ matrix.target }})`, `needs: [create-release]`, `permissions: { contents: write }`, `runs-on: ${{ matrix.os }}`, `defaults: { run: { shell: bash } }`.
- [ ] **Step 2:** Add the gating `if:` line: `if: always() && (needs.create-release.result == 'success' || needs.create-release.result == 'skipped')`. Add an inline YAML comment immediately above explaining that `skipped` is required so the dispatch path runs when `create-release` is gated off.
- [ ] **Step 3:** Add `strategy.fail-fast: false`.
- [ ] **Step 4:** Add `strategy.matrix.include:` with three entries — macOS arm64, Ubuntu x86_64, Windows x86_64 — each with the four matrix variables (`os`, `target`, `bin_suffix`, `checksum_cmd`) per the spec table.

## Task 4: `build` job steps (build + test)

- [ ] **Step 1:** Add `uses: actions/checkout@v4` step.
- [ ] **Step 2:** Add step `name: Add Rust target` running `rustup target add ${{ matrix.target }}`.
- [ ] **Step 3:** Add step `name: Build` running `cargo build --release --target ${{ matrix.target }}`.
- [ ] **Step 4:** Add step `name: Test` running `cargo test --release --target ${{ matrix.target }}` (no `if:` — every entry is native and tests).

## Task 5: Rename and checksum steps

- [ ] **Step 1:** Add step `name: Rename binary` running `cp "target/${{ matrix.target }}/release/select-mirror${{ matrix.bin_suffix }}" "select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}"`.
- [ ] **Step 2:** Add step `name: Generate sha256` running `${{ matrix.checksum_cmd }} "select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}" > "select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}.sha256"` from the working directory so the embedded filename has no path prefix.

## Task 6: Conditional upload steps

- [ ] **Step 1:** Add step `name: Upload to GitHub Release` with `if: github.event_name == 'push'`. Body: `gh release upload "${{ github.ref_name }}" "select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}" "select-mirror-${{ matrix.target }}${{ matrix.bin_suffix }}.sha256" --repo "${{ github.repository }}"`. `env: GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`.
- [ ] **Step 2:** Add step `name: Upload as workflow artifact` with `if: github.event_name == 'workflow_dispatch'`. Use `uses: actions/upload-artifact@v4` with `name: select-mirror-${{ matrix.target }}` and `path:` listing both files (multi-line block scalar).

## Task 7: Local validation

- [ ] **Step 1:** Run `openspec validate add-release-workflow --strict` and resolve any reported issues.
- [ ] **Step 2:** If `actionlint` is installed locally (`which actionlint`), run `actionlint .github/workflows/release.yml` and fix any warnings.

## Task 8: End-to-end verification on the live repo

- [ ] **Step 1:** Commit the workflow file to a feature branch and push.
- [ ] **Step 2:** Trigger `workflow_dispatch` from the Actions UI on the feature branch. Wait for completion. Confirm all three matrix entries are green and the run page shows three artifacts (`select-mirror-aarch64-apple-darwin`, `select-mirror-x86_64-unknown-linux-gnu`, `select-mirror-x86_64-pc-windows-msvc`).
- [ ] **Step 3:** Download each artifact, place binary + sidecar in the same directory, and verify with `shasum -c <bin>.sha256` (macOS download) or `sha256sum -c <bin>.sha256` (Linux/Windows downloads). All three must succeed.
- [ ] **Step 4:** Confirm via `gh release list` that the dispatch run did not create any GitHub Release.
- [ ] **Step 5:** Merge the feature branch to `main`. Push tag `v0.1.0`. Wait for the workflow to complete. Confirm via `gh release view v0.1.0` that the Release exists with auto-generated notes and exactly six assets (3 binaries + 3 sidecars).

---

## Commit points

- After Task 6: one commit `chore(ci): add release workflow` containing only `.github/workflows/release.yml`.
- After Task 7 if local validation surfaced changes: amend or follow-up commit.
- After Task 8 if any tweaks were needed during E2E verification: separate commit per fix.
