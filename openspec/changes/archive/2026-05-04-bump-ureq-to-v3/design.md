## Context

The current implementation uses ureq 2, which does not automatically read `*_PROXY` / `NO_PROXY` environment variables. This gap is bridged by three helper functions (`proxy_from_env`, `host_from_url`, `host_matches_no_proxy`) and the `build_agent` wrapper (~95 lines total). ureq 3 ships this logic natively via `Proxy::try_from_env` and automatic per-agent env-var reading, making the hand-rolled code redundant.

## Goals / Non-Goals

**Goals:**
- Upgrade `ureq` to v3
- Delete all manual proxy/NO_PROXY helper code
- Preserve identical observable behavior (proxy routing, NO_PROXY bypass, timeout, direct-connection fallback)

**Non-Goals:**
- Changing CLI interface or output format
- Adding new proxy configuration options
- Altering timeout semantics (per-probe timeout is unchanged)

## Decisions

### Inline agent creation into `probe()`, delete `build_agent()`
`build_agent()` existed solely to wire in the proxy. Without manual proxy wiring, it becomes a one-liner not worth a separate function. The agent is created fresh per `probe()` call (one thread per mirror), so inlining is safe and reduces indirection.

*Alternative considered*: Keep `build_agent()` as a thin wrapper — rejected as unnecessary indirection.

### Use `timeout_global` on the agent instead of per-request `.timeout()`
ureq 3 removes the per-request `.timeout()` method; timeout is now an agent-level config via `.timeout_global(Some(duration))`. Since each `probe()` call creates a fresh agent for a single request, `timeout_global` is semantically identical to the old per-request timeout.

*Alternative considered*: `timeout_per_call` — also valid, but `timeout_global` is the closer semantic match to "abort after N seconds regardless".

### Delete `proxy_from_env`, `host_from_url`, `host_matches_no_proxy` entirely
ureq 3's default agent reads the same env vars in the same priority order (`ALL_PROXY` → `HTTPS_PROXY` → `HTTP_PROXY`, lowercase variants) and handles `NO_PROXY` dot-suffix matching internally. Keeping parallel implementations would create a drift risk if ureq 3's behavior evolves.

### Delete the 5 unit tests covering removed helpers
The tests covered implementation internals (`host_from_url` parsing, `host_matches_no_proxy` matching). Those internals no longer exist. Behavioral proxy coverage continues through integration tests.

## Risks / Trade-offs

- **NO_PROXY matching parity** → ureq 3 uses the same dot-suffix and exact-match rules; the behavioral contract in `aware-sandbox` spec is preserved.
- **socks5h:// now supported** → ureq 3 supports SOCKS5; the old "fall through on socks5h://" scenario no longer applies. This is an improvement (more proxy schemes work), not a regression.
- **Other ureq 3 breaking changes** → our API surface is minimal: `Agent`, `.get()`, `.call()`. The only structural change is the timeout location, covered above.

## Migration Plan

1. `Cargo.toml`: `ureq = "2"` → `ureq = "3"`
2. `probe()`: remove `build_agent` call; inline `Agent::config_builder().timeout_global(Some(...)).build().into()`; remove `.timeout()` from request
3. Delete `build_agent()`, `proxy_from_env()`, `host_from_url()`, `host_matches_no_proxy()`
4. Delete 5 unit tests: `host_from_url_extracts_simple_host`, `host_from_url_strips_port`, `host_from_url_handles_bracketed_ipv6`, `host_from_url_returns_none_without_scheme`, `no_proxy_matcher_exact_and_suffix`
5. `cargo test` — all remaining tests must pass

**Rollback**: `git revert` on `Cargo.toml` and `src/main.rs`.
