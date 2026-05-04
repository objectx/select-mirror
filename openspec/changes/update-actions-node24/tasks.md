## 1. Bump action versions in release workflow

- [x] 1.1 Edit `.github/workflows/release.yml` line 54: change `actions/checkout@v4` to `actions/checkout@v6`.
- [x] 1.2 Edit `.github/workflows/release.yml` line 79: change `actions/upload-artifact@v4` to `actions/upload-artifact@v7`.
- [x] 1.3 Confirm no other `uses:` lines reference deprecated versions.

## 2. Add Dependabot configuration

- [x] 2.1 Create `.github/dependabot.yml` with the `github-actions` ecosystem entry, monthly cadence, and `chore(ci)` commit-message prefix.
- [x] 2.2 Verify the YAML parses (e.g. via `python -c "import yaml; yaml.safe_load(open('.github/dependabot.yml'))"`).

## 3. Verify the workflow still works

- [x] 3.1 Run `openspec validate update-actions-node24` — must report valid.
- [x] 3.2 Trigger a `workflow_dispatch` run on the release workflow and confirm: all three matrix entries succeed, no Node 20 deprecation warning appears, and `actions/upload-artifact@v7` produces the per-target artifact pair.
- [x] 3.3 Confirm Dependabot is enabled for the repository in GitHub repo settings (it activates automatically once the config file is committed to the default branch).

## 4. Update the spec to match

- [ ] 4.1 After the workflow change merges to `main`, run `openspec archive update-actions-node24` to apply the `release-build` spec delta (updates the `actions/upload-artifact` version reference from `@v4` to `@v7`).
