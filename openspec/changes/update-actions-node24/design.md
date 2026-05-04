## Context

`.github/workflows/release.yml` pins two third-party actions:

- Line 54: `actions/checkout@v4` — runs on Node 20
- Line 79: `actions/upload-artifact@v4` — runs on Node 20

GitHub has deprecated the Node 20 runtime for actions; every workflow run now emits a deprecation warning. Latest GA majors are `actions/checkout@v6` and `actions/upload-artifact@v7`, both running on Node 24. The repo currently has no Dependabot configuration, so the only signal of upstream deprecation is GitHub's runtime warning during release builds.

This is a maintenance bump. The workflow's external contract (per the `release-build` capability spec) — multi-target binary artifacts plus sha256 sidecars uploaded to a tagged GitHub Release — is unaffected.

## Goals / Non-Goals

**Goals:**

- Eliminate Node 20 deprecation warnings from `release.yml`.
- Pick up active upstream maintenance for `actions/checkout` and `actions/upload-artifact`.
- Automate detection of future major-version drift via Dependabot for the `github-actions` ecosystem.

**Non-Goals:**

- Behavioral changes to release outputs (binary names, sha256 algorithm, artifact layout).
- Changes to runner OS pins (`ubuntu-24.04`, `windows-2025`, `macos-latest`).
- SHA pinning of actions (out of scope for this repo's threat model).
- Dependabot for `cargo` or other ecosystems (separate concern).

## Decisions

### D1. Bump `actions/checkout@v4 → @v6`

Floating-major ref. Latest GA at proposal time is `v6.0.2`.

- **Why this version**: v5 transitioned to Node 24; v6 added a credential-storage refinement (uses `$RUNNER_TEMP` instead of local git config) which is the current GA line.
- **Why floating major**: Patch updates within v6 ship transparently. The repo passes zero inputs to checkout, so minor input changes can't bite us.
- **Alternatives**: Exact tag `@v6.0.2` (rejected — adds Dependabot churn for trivial patch bumps); SHA pin (rejected — threat model doesn't justify).

### D2. Bump `actions/upload-artifact@v4 → @v7`

Floating-major ref. Latest GA at proposal time is `v7.0.1`.

- **Why this version**: v6 made Node 24 the default runtime; v7 converted internals to ESM and added an opt-in `archive: false` parameter for unzipped uploads. Defaults are unchanged from v4.
- **Why floating major**: The workflow passes only `name` and a multi-line `path` — both stable across v4→v7. The new `archive` knob is opt-in.
- **Alternatives**: Same as D1 — exact tag and SHA pin both rejected for the same reasons.

### D3. Add `.github/dependabot.yml` for `github-actions`

Monthly cadence, `chore(ci)` commit prefix, default PR limit.

- **Why monthly**: `release.yml` runs only on tag push. Weekly Dependabot noise has no payoff for a release-only workflow.
- **Why `chore(ci)` prefix**: Matches the existing commit convention (`5cc09e9 chore(ci): add release workflow`).
- **Alternatives**: Weekly (rejected — too noisy for the cadence at which this project releases); grouping into a single PR (rejected — only two actions, not worth the config surface).

### D4. No spec delta

The `release-build` capability spec describes the workflow's outputs and contract. Bumping action runtime versions doesn't change what's produced — same binaries, same sha256 sidecars, same upload destinations. Therefore no entries under `specs/`.

## Risks / Trade-offs

- **[Risk] Floating major picks up a future v6.x.y / v7.x.y change that breaks our usage** → Mitigation: We pass minimal inputs; first-party `actions/*` rarely break minors. If a regression occurs, pinning to a known-good tag is a one-line revert.
- **[Risk] Dependabot opens noisy PRs the team has to triage** → Mitigation: Monthly cadence and only one ecosystem keeps volume to roughly one PR per quarter in steady state.
- **[Risk] `actions/checkout@v6` Docker container scenario change (credentials moved to `$RUNNER_TEMP`)** → Not applicable: this workflow runs `cargo`/`rustup`/`gh` directly on the runner host, not inside a Docker container action.
- **[Risk] `actions/upload-artifact@v7` ESM conversion breaks somehow** → Not applicable: ESM is internal to the action; consumer-facing inputs and outputs unchanged. Default `archive: true` (zipped) preserves v4 behavior.

## Migration Plan

1. Edit two `uses:` lines in `.github/workflows/release.yml`.
2. Add `.github/dependabot.yml` with the `github-actions` ecosystem entry.
3. Trigger a `workflow_dispatch` run to confirm the workflow still completes successfully and the deprecation banner is gone.
4. Tag a real release on a future date to confirm the `push: tags` path still produces the expected release assets.

**Rollback**: Revert the commit. The workflow is the only artifact touched, and `actions/checkout@v4` / `actions/upload-artifact@v4` remain published indefinitely.

## Open Questions

None. All decisions confirmed during explore-mode brainstorming.
