## Design Summary

GitHub deprecated the Node 20 runtime for actions; the workflow at `.github/workflows/release.yml` pins `actions/checkout@v4` (line 54) and `actions/upload-artifact@v4` (line 79), both of which run on Node 20 and emit deprecation warnings on every workflow run. Bump both to their current major (`@v6` for checkout, `@v7` for upload-artifact) using floating-major refs, and bundle a Dependabot config so `github-actions` updates surface automatically next time.

## Alternatives Considered

### Option A: Floating-major refs + Dependabot

- **Approach**: Update `actions/checkout@v4 → @v6` and `actions/upload-artifact@v4 → @v7`. Add `.github/dependabot.yml` for the `github-actions` ecosystem on a monthly cadence with `chore(ci)` commit prefix.
- **Pros**:
  - Patch updates within a major are picked up automatically (security fixes, minor improvements).
  - Dependabot raises a PR the next time a major bump is needed — no more manual deprecation chase.
  - Conventional in the GitHub Actions ecosystem; minimal config.
- **Cons**:
  - A future v6.x.y minor change could in theory break behavior without us noticing in advance (low risk for first-party `actions/*`).
- **Why not chosen**: This *is* the chosen approach.

### Option B: Exact-tag pins (`@v6.0.2`, `@v7.0.1`)

- **Approach**: Pin to the precise patch tag and rely on Dependabot to bump it.
- **Pros**: Reproducible; no surprise changes from upstream re-tagging.
- **Cons**: Adds churn — every patch becomes a Dependabot PR. Overkill for a small release-only workflow on a personal CLI.
- **Why not chosen**: Cost-benefit tilts toward floating major for this repo's size and risk profile.

### Option C: Full SHA pinning

- **Approach**: Pin each action to a 40-character commit SHA (supply-chain hardened).
- **Pros**: Defends against tag re-pointing / compromised maintainer accounts.
- **Cons**: Heavyweight; requires Dependabot in `cooldown`-aware mode to be ergonomic; unjustified threat model for this repo.
- **Why not chosen**: Threat model doesn't warrant it; reserved for higher-stakes pipelines.

## Agreed Approach

Option A — floating major refs (`@v6` / `@v7`) plus Dependabot for the `github-actions` ecosystem (monthly cadence, `chore(ci)` commit prefix).

This matches the repo's lightweight tooling posture and ensures the next deprecation cycle is caught by automation rather than manual review.

## Key Decisions

- **Bump targets**: `actions/checkout@v4 → @v6` (latest GA v6.0.2), `actions/upload-artifact@v4 → @v7` (latest GA v7.0.1). Both run on Node 24.
- **Pin style**: Floating major. Patch and minor updates within a major are picked up implicitly.
- **Dependabot cadence**: Monthly. Workflow runs only on tag push — weekly noise has no payoff.
- **Dependabot commit prefix**: `chore(ci)` to match existing convention (e.g., commit `5cc09e9 chore(ci): add release workflow`).
- **Minimal spec delta**: The `release-build` spec literally pins `actions/upload-artifact@v4` inside the "Conditional artifact upload by event" requirement, so a single MODIFIED delta is needed to update that version reference to `@v7`. No scenarios or other requirements change. (`actions/checkout` is not version-pinned in the spec text, so it needs no delta.)
- **Design doc kept lightweight**: No architectural choice — design boils down to "edit two version refs, add a small dependabot file." `design.md` records context, decisions, and rollback so the change is self-contained, but does not invent new structure.
- **Verified non-breaking**:
  - `checkout` v6 changes credential storage location (`$RUNNER_TEMP` vs local git config) — affects only Docker container action scenarios; this workflow runs `cargo`/`rustup` directly on the host.
  - `upload-artifact` v7 introduced ESM internals and an opt-in `archive: false` knob; default zip behavior is unchanged. The workflow passes only `name` + multi-line `path`, both stable across majors.

## Open Questions

None. Decisions confirmed during explore-mode discussion.
