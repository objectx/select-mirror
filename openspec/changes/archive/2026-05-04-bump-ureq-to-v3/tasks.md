# Implementation Milestones

## Milestones

- [x] **Milestone 1: Bump ureq dependency**
  Update `Cargo.toml` to use ureq 3 and verify the project compiles with the new major version.

- [x] **Milestone 2: Migrate probe() to ureq 3 API**
  Replace `AgentBuilder`-based agent creation with `Agent::config_builder().timeout_global()`, moving timeout from the request to the agent. Satisfies the timeout requirement in `aware-sandbox`.

- [x] **Milestone 3: Remove manual proxy helpers**
  Delete `build_agent()`, `proxy_from_env()`, `host_from_url()`, and `host_matches_no_proxy()`, and their unit tests. Proxy and NO_PROXY handling is now delegated to ureq 3 natively, satisfying all requirements in `aware-sandbox`.

- [x] **Milestone 4: Verify all tests pass**
  Confirm the remaining test suite (integration tests + surviving unit tests) passes clean with no regressions.
