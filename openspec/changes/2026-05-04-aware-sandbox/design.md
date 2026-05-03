## Context

`select-mirror` is a single-binary Rust CLI built on `ureq 2.x`. Its only network operation is `probe()`, which fires a `GET` against `<mirror><probe-path>` and reports elapsed time. Outside a sandbox, ureq's default behaviour — direct connection, system resolver — is fine.

Inside Claude Code's sandbox (and similar proxy-filtered environments), the situation is different. The sandbox injects:

```
HTTP_PROXY=http://localhost:<port>
HTTPS_PROXY=http://localhost:<port>
ALL_PROXY=socks5h://localhost:<port>
NO_PROXY=localhost,127.0.0.1,::1,*.local,...
```

…and blocks direct DNS for arbitrary external hosts. Only domains in `sandbox.network.allowedDomains` are forwarded by the proxy.

Two important properties of `ureq 2.x` shape this design:

1. **ureq does not auto-read `*_PROXY` env vars.** Without an explicit `ureq::Proxy`, ureq attempts a direct connection and resolves the target via `getaddrinfo()`, which fails with `EAI_NONAME` in the sandbox.
2. **With an explicit `ureq::Proxy`, ureq resolves only the proxy host.** Verified at `ureq` 2.12 `src/stream.rs`, `connect_host`: `netloc` is `proxy.server:proxy.port` when a proxy is set, and `hostname:port` only when no proxy is configured. The target hostname is sent to the proxy in absolute-form for HTTP or via `CONNECT` for HTTPS — never to the local resolver.

So the fix is purely about reading the env vars and configuring ureq's existing `Proxy` machinery. No new client, no hand-rolled HTTP, no new dependencies.

## Goals / Non-Goals

**Goals:**

- Work transparently inside Claude Code's sandbox using only standard `*_PROXY` / `NO_PROXY` env vars — no new flags, no per-environment branches.
- Zero new dependencies. Reuse `ureq 2.x`'s native `Proxy` support.
- Preserve current behaviour outside the sandbox (no env vars set → direct connection, exactly as before).
- Keep integration tests against `127.0.0.1` working without modification.
- Tolerate sandbox-injected values that ureq cannot parse (specifically `ALL_PROXY=socks5h://…`).

**Non-Goals:**

- SOCKS proxy support. The sandbox always provides an HTTP proxy alongside the SOCKS one; falling through to `HTTP_PROXY` is sufficient.
- Per-request proxy authentication beyond what `ureq::Proxy::new` already accepts.
- Adding a `--proxy` CLI flag. The standard env-var contract is enough.
- Glob support in NO_PROXY beyond the `*.suffix` and bare `suffix` forms used by Claude Code's sandbox and ordinary loopback rules.
- Migrating to `ureq 3.x`. Out of scope for this change.

## Decisions

### Decision 1: Read `*_PROXY` env vars manually and pass to `ureq::Proxy`

**Choice**: Construct `ureq::Proxy` from the first non-empty, parseable env var and attach it via `AgentBuilder::proxy(...)`.

**Why**: This is the ergonomic seam ureq already supports. The `Proxy` is the contract that tells ureq "send absolute-form HTTP to this host" or "use CONNECT through this host" — both of which already work in 2.12. We just have to feed the configuration in.

**Alternatives considered**:

- *Hand-rolled HTTP/1.1 to the proxy socket* (commit `d90c3ec`, never landed) — rejected: bypasses ureq's HTTPS support, loses redirect/connect handling, duplicates logic ureq already implements correctly.
- *Switch to `reqwest`* — rejected: reqwest reads `*_PROXY` natively but pulls in `tokio` and a much larger transitive tree for a tiny CLI. ureq is sufficient once configured.
- *Wait for `ureq 3.x`* — rejected: 3.x is not a drop-in upgrade; the migration is a separate change. The 2.x fix lands today.

### Decision 2: Env-var priority `ALL_PROXY → HTTPS_PROXY → HTTP_PROXY`

**Choice**: Try `ALL_PROXY`, `all_proxy`, `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy` in that order. First non-empty, parseable value wins.

**Why**: Mirrors `ureq::Proxy::try_from_system` and the order documented by curl. `ALL_PROXY` is the most-specific override; `HTTP_PROXY` is the broadest fallback. Lowercase variants are honoured because POSIX environments commonly export lowercase forms.

**Alternatives considered**:

- *Match the ureq private helper directly via reflection or feature flag* — rejected: `try_from_system` is `pub(crate)`, not exposed. Reading env vars ourselves is ten lines of safe code.
- *Use only `HTTP_PROXY`* — rejected: ignoring `ALL_PROXY` would mean shifting to silent failure when only `ALL_PROXY` is set (a real configuration in some CI setups).

### Decision 3: Fall through on `ureq::Proxy::new` parse failure

**Choice**: If parsing one variable fails, try the next instead of giving up.

**Why**: Claude Code's sandbox sets `ALL_PROXY=socks5h://localhost:<port>`. `ureq::Proxy::new` does not recognize `socks5h://` (only `socks4`, `socks4a`, `socks`, `socks5`). Without fall-through, sandboxed runs would silently disable the proxy for the rest of the priority chain and fall back to direct connection, which then fails. Falling through to `HTTPS_PROXY` / `HTTP_PROXY` (both `http://localhost:<port>`) is the right answer and keeps the code free of sandbox-specific knowledge.

**Alternatives considered**:

- *Hard-fail on parse error* — rejected: would make the sandbox case worse than today (currently no proxy is configured at all, so we'd be regressing into a louder failure mode).
- *Enable the `socks-proxy` ureq feature* — rejected: adds a transitive dependency for a path we never need (HTTP proxy is always available beside SOCKS in the sandbox, and falling through gets us there).

### Decision 4: NO_PROXY matcher: exact equality + dot-suffix

**Choice**: Split NO_PROXY by `,`, strip each entry's leading `*.` and `.`, then bypass the proxy if the target host equals the entry OR ends with `.<entry>`.

**Why**: Covers every value Claude Code's sandbox writes (`localhost`, `127.0.0.1`, `::1`, `*.local`) and the only target select-mirror reaches in tests (`127.0.0.1`). The full "RFC-ish" NO_PROXY universe (CIDR ranges, port suffixes, leading wildcards anywhere) is not used here and would be dead code in this CLI.

**Alternatives considered**:

- *Pull a NO_PROXY crate (`hyper-proxy`, `no_proxy_aware`, etc.)* — rejected: ten lines of code vs. another transitive tree.
- *Hardcode loopback bypass without consulting NO_PROXY* — rejected: NO_PROXY is the standard signal; respecting it costs nothing and is correct.
- *Ignore NO_PROXY entirely* — rejected: would cause integration tests to fail when run inside any environment that has `HTTP_PROXY` set (CI proxies, dev machines on corp networks). Honouring NO_PROXY makes the change safe to land everywhere.

### Decision 5: Build the agent per probe call

**Choice**: `probe()` calls `build_agent(&url)` each time, returning a fresh `ureq::Agent` configured with or without a proxy depending on whether the URL matches NO_PROXY.

**Why**: ureq's `Proxy` is per-agent, not per-request. Probes can target a mix of proxy-eligible mirrors and NO_PROXY-bypassed targets in the same run (e.g. an integration test that mixes a localhost mock with an external URL). Per-probe agent construction is cheap relative to a network probe and keeps `probe()`'s public signature untouched.

**Alternatives considered**:

- *One global agent built in `main`, no NO_PROXY* — rejected: would route `127.0.0.1` test traffic through the proxy when `HTTP_PROXY` happens to be set in the environment (corp dev machines, CI).
- *Two agents — one with proxy, one without — chosen per-probe* — rejected: marginally more efficient, materially more state. Per-probe construction is well below the noise floor of a network call.

### Decision 6: Don't change `probe()`'s signature

**Choice**: Keep `probe(mirror, probe_path, timeout_secs) -> Option<f64>`. Encapsulate proxy selection inside.

**Why**: `probe()` is called from two paths — the cache-hit short-circuit and the parallel probe-all flow. Changing its signature would ripple unnecessarily. The agent is built from the URL, which `probe()` already constructs.

### Decision 7: No new flag

**Choice**: Behaviour is entirely env-driven. No `--proxy`, no `--no-proxy`.

**Why**: The standard `*_PROXY` contract is what every consumer (curl, git, apt, Docker) already speaks. Adding a flag would be a new CLI surface to maintain and document with no carrying value.

## Risks / Trade-offs

- **A user with unrelated `HTTP_PROXY` set will now route `select-mirror` traffic through it.** → Mitigated by NO_PROXY support: typical loopback-bypass configurations Just Work. Documented in README so the change is discoverable.
- **`ureq::Proxy::new` parse rules may shift between minor versions.** → Mitigated by fall-through: unknown values silently degrade to the next priority var. We pin `ureq = "2"` (any 2.x) and the contract has been stable across 2.x.
- **NO_PROXY matcher is intentionally minimal.** → Acceptable: covers every real value used today. If a future need pushes us toward CIDR or port-aware matching, we revisit. Not now.
- **Per-probe agent construction has a small allocation cost.** → Bounded by mirror count (≤ a few dozen in practice); dominated by the network call. Below the noise floor.
- **Integration tests assume `127.0.0.1` is in NO_PROXY by convention or unset proxy env vars in the test runner's environment.** → Stated explicitly; no test changes required because most CI envs do not set `HTTP_PROXY` and those that do follow the loopback-bypass convention.

## Migration Plan

This is a behaviour-additive change. Users without `*_PROXY` set see no change. Users running under Claude Code's sandbox (or any other proxy-filtered environment) get working external probes for free.

No schema/data migration. No version-bump policy decision in this change.

Rollback: revert the commit. The cache file format is unaffected.

For the consumer running tests under the sandbox, the consumer's `.claude/settings.json` must include `"sandbox": { "network": { "allowLocalBinding": true } }` so the integration test suite can `bind("127.0.0.1:0")`. This is documented in `findings-about-claude-sandboxing.md` and is a sandbox-config concern, not a runtime change to `select-mirror` itself.

## Open Questions

None. Verified end-to-end inside the `local-ubuntu` consumer's sandbox.
