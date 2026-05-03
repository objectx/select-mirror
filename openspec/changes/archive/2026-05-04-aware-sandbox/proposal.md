## Why

`select-mirror` cannot reach external mirrors when invoked from a network-filtered sandbox (e.g. Claude Code's `sandbox.enabled: true` mode). The sandbox routes all outbound traffic through a local HTTP/SOCKS proxy and blocks direct DNS for arbitrary hosts. `ureq 2.x` — unlike `libcurl` — does not auto-detect `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` environment variables, so it attempts a direct connection, calls `getaddrinfo()` on the target, and fails with `EAI_NONAME` regardless of whether the target appears in the sandbox's `allowedDomains`. The tool is unusable from inside any consumer that runs it under such a sandbox (verified failure in the `local-ubuntu` client project).

## What Changes

- Probe reads `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` (and lowercase variants) from the environment and constructs a `ureq::Proxy` attached to the agent via `AgentBuilder::proxy(...)`. With a proxy configured, ureq resolves only the proxy host; the target hostname is sent to the proxy in absolute-form (HTTP) or via `CONNECT` (HTTPS) and is never passed to the local resolver.
- Probe honours `NO_PROXY` (and `no_proxy`) with a minimal exact-match plus dot-suffix matcher, so loopback targets such as `127.0.0.1`, `::1`, and `*.local` continue to bypass the proxy. Integration tests against `127.0.0.1` keep working without changes.
- Parse failures fall through to the next env var in priority order. A sandbox that exports `ALL_PROXY=socks5h://…` (a scheme `ureq::Proxy::new` does not recognize) gracefully falls back to `HTTPS_PROXY` or `HTTP_PROXY` instead of erroring.
- No new dependencies (the `socks-proxy` ureq feature is **not** enabled). No new flags. No data migration. When none of the `*_PROXY` variables are set, behaviour is unchanged.

## Capabilities

### New Capabilities
- `aware-sandbox`: Make `select-mirror` work transparently inside a network-filtered sandbox by honouring the standard proxy and no-proxy environment variables, without introducing new flags or dependencies.

### Modified Capabilities
<!-- None — `mirror-selection-cache` is unaffected; cache hits and probe-all both go through the same proxy-aware probe. -->

## Impact

- **Code**: `src/main.rs` adds `build_agent`, `proxy_from_env`, `host_from_url`, `host_matches_no_proxy`. `probe()` constructs a per-URL agent so the NO_PROXY decision can vary by target.
- **Tests**: `src/main.rs` unit tests for `host_from_url` (simple host, port, bracketed IPv6, no-scheme) and `host_matches_no_proxy` (exact, dot-suffix, suffix non-match). Integration tests in `tests/cli.rs` are unchanged and continue to pass because `127.0.0.1` is bypassed via NO_PROXY.
- **Dependencies**: none added. `ureq 2.x` already supports HTTP `CONNECT` and absolute-form HTTP through a `Proxy`. `socks-proxy` feature is intentionally not enabled.
- **Sandbox configuration**: documents the `allowLocalBinding: true` requirement for running the integration test suite under the sandbox (the suite binds `127.0.0.1:0` for mock servers). This belongs in `.claude/settings.json` of the consumer running the tests, not in this repo's runtime behaviour.
- **Consumers**: tools that already set `HTTP_PROXY` for unrelated reasons will now route `select-mirror` traffic through that proxy. NO_PROXY semantics are honoured, so sane configurations (loopback bypass) are unaffected. Documented in `README.md` for discoverability.
- **Earlier ad-hoc workaround**: commit `d90c3ec` (never on `main`) hand-rolled HTTP/1.1 directly to the proxy socket. Superseded by this change; left dangling intentionally as a record of the rejected approach.
