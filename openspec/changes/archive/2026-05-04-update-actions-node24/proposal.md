## Why

`.github/workflows/release.yml` pins `actions/checkout@v4` and `actions/upload-artifact@v4`, both of which run on the deprecated Node 20 runtime. Every release build emits GitHub's deprecation banner, and the repo has no automated mechanism for surfacing the next major drift. Bumping now eliminates the warning, picks up active upstream maintenance, and adds Dependabot so we're notified the next time this happens instead of finding it in CI logs.

## What Changes

**`actions/checkout` version**
- From: `actions/checkout@v4` (Node 20 runtime)
- To: `actions/checkout@v6` (Node 24 runtime, latest GA `v6.0.2`)
- Reason: Node 20 deprecated by GitHub Actions.
- Impact: Non-breaking. Workflow passes no inputs to the action; v6's credential-storage refinement only affects Docker container action scenarios, which this workflow does not use.

**`actions/upload-artifact` version**
- From: `actions/upload-artifact@v4` (Node 20 runtime)
- To: `actions/upload-artifact@v7` (Node 24 runtime, latest GA `v7.0.1`)
- Reason: Node 20 deprecated by GitHub Actions.
- Impact: Non-breaking. The workflow passes only `name` + multi-line `path`, both stable across v4→v7. v7's ESM conversion is internal; the new `archive: false` parameter is opt-in and unused here.

**Dependabot configuration**
- Add: `.github/dependabot.yml` declaring the `github-actions` ecosystem with monthly cadence and `chore(ci)` commit-message prefix.
- Reason: Surface future GitHub Actions deprecations and major bumps automatically, in line with the repo's existing conventional-commit style.
- Impact: Non-breaking. Adds a config file; raises Dependabot PRs when upstream updates appear.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `release-build`: the existing "Conditional artifact upload by event" requirement names `actions/upload-artifact@v4` by exact version. That literal pin must be updated to `actions/upload-artifact@v7` to match the new workflow. No scenarios change — only the version reference inside the requirement text.

## Impact

- **Files modified**: `.github/workflows/release.yml` (two `uses:` lines).
- **Files added**: `.github/dependabot.yml`.
- **APIs / contracts**: None affected.
- **Dependencies**: GitHub Actions runtime moves from Node 20 → Node 24 for both bumped actions; matches GitHub's current default. No new repo-level dependencies introduced.
- **CI / CD**: Existing `push: tags` and `workflow_dispatch` paths continue to function identically. Dependabot will begin opening PRs against `.github/workflows/*.yml` when upstream majors release.
