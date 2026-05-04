## Why

ureq 2 does not automatically read proxy environment variables, requiring a manual implementation of `proxy_from_env`, `host_from_url`, and `host_matches_no_proxy`. ureq 3 handles `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and `NO_PROXY` natively, making ~95 lines of hand-rolled proxy logic redundant.

## What Changes

- Bump `ureq` dependency from `"2"` to `"3"` in `Cargo.toml`
- Replace `ureq::AgentBuilder::new()` with `ureq::Agent::config_builder()` and move timeout from per-request to per-agent via `timeout_global`
- Delete `build_agent()` — inlined into `probe()`
- Delete `proxy_from_env()` — ureq 3 reads env vars automatically
- Delete `host_from_url()` — only fed `build_agent`
- Delete `host_matches_no_proxy()` — ureq 3 owns `NO_PROXY` matching
- Delete 5 unit tests covering the deleted functions

## Capabilities

### New Capabilities
<!-- none — this is a pure dependency upgrade with code deletion -->

### Modified Capabilities
- `aware-sandbox`: Proxy support no longer requires manual env-var reading; behaviour is unchanged but now delegated to ureq 3 internals.

## Impact

- `Cargo.toml`: single version bump
- `src/main.rs`: ~95 lines removed, `probe()` simplified
- No public API or CLI interface changes
- All existing integration tests continue to apply; 5 unit tests for deleted helpers are removed
