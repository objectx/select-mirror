## MODIFIED Requirements

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
