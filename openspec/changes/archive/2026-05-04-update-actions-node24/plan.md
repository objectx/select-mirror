# Update GitHub Actions to Node 24 Implementation Plan

**Goal:** Replace deprecated Node 20 action references in `.github/workflows/release.yml` with their current Node 24 GA majors, and add Dependabot for the `github-actions` ecosystem so future drift surfaces automatically.

**Architecture:** Two `uses:` line edits in the existing release workflow plus a new `.github/dependabot.yml` config file. No application code or runtime behavior changes; the spec delta updates the `release-build` capability text to match the new version pin.

**Tech Stack:** GitHub Actions YAML, Dependabot v2 config schema, OpenSpec (for the spec delta).

---

## Task 1: Bump action versions in `.github/workflows/release.yml`

- [ ] **Step 1:** Open `.github/workflows/release.yml`. At line 54, change `- uses: actions/checkout@v4` to `- uses: actions/checkout@v6`.
- [ ] **Step 2:** At line 79, change `uses: actions/upload-artifact@v4` to `uses: actions/upload-artifact@v7`.
- [ ] **Step 3:** Run `grep -nE 'uses: actions/' .github/workflows/release.yml` and confirm the two `uses:` lines now read `@v6` and `@v7`, and that no other `uses:` lines reference an older version.
- [ ] **Step 4:** Run `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` to confirm the YAML still parses cleanly.

## Task 2: Add `.github/dependabot.yml`

- [ ] **Step 1:** Create `.github/dependabot.yml` with this content:
  ```yaml
  version: 2
  updates:
    - package-ecosystem: "github-actions"
      directory: "/"
      schedule:
        interval: "monthly"
      commit-message:
        prefix: "chore(ci)"
  ```
- [ ] **Step 2:** Run `python -c "import yaml; yaml.safe_load(open('.github/dependabot.yml'))"` to confirm the YAML is valid.

## Task 3: Validate the OpenSpec change

- [ ] **Step 1:** Run `openspec validate update-actions-node24`. Expected output: `Change 'update-actions-node24' is valid`.
- [ ] **Step 2:** Run `openspec status --change update-actions-node24` and confirm every artifact in `applyRequires` is `done`.

## Task 4: Smoke-test the workflow

- [ ] **Step 1:** Commit the workflow + dependabot changes with message `chore(ci): bump deprecated GitHub Actions to Node 24 majors` and push to a feature branch.
- [ ] **Step 2:** Trigger `workflow_dispatch` for the release workflow on that branch via the GitHub Actions UI or `gh workflow run release.yml --ref <branch>`.
- [ ] **Step 3:** Inspect the run logs for all three matrix entries and confirm:
  - No `Node.js 20 actions are deprecated` warning appears.
  - The `actions/upload-artifact@v7` step succeeds and produces a downloadable artifact named `select-mirror-<target>` containing the binary plus its `.sha256` sidecar.
  - The `actions/checkout@v6` step completes successfully on each runner OS.
- [ ] **Step 4:** Confirm Dependabot is enabled for the repository (Settings → Code security → Dependabot version updates → "Active") once the config is on the default branch. No PRs are expected on day one — the next one fires when an upstream major drops.

## Task 5: Archive after merge

- [ ] **Step 1:** After the PR merges to `main`, run `openspec archive update-actions-node24`. This applies the `release-build` spec delta: the `actions/upload-artifact@v4` reference inside the "Conditional artifact upload by event" requirement is updated to `@v7`.
- [ ] **Step 2:** Commit the archived change directory and updated spec with message `docs(openspec): archive update-actions-node24, sync release-build spec`.
