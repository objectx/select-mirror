# Claude Code Sandbox — Findings & select-mirror Fix

## Background

When Claude Code runs with `sandbox.enabled: true`, every shell command is
wrapped in a macOS sandbox profile.  Network filtering is implemented via a
local HTTP/HTTPS proxy that Claude Code injects into the environment:

```
HTTP_PROXY=http://localhost:<port>
HTTPS_PROXY=http://localhost:<port>
ALL_PROXY=socks5h://localhost:<port>
NO_PROXY=localhost,127.0.0.1,::1,*.local,...
```

Only domains listed in `sandbox.network.allowedDomains` are forwarded by the
proxy.  Requests to unlisted domains are dropped.

## Root Cause of select-mirror Failure

`select-mirror` uses `ureq 2.x` as its HTTP client.  Unlike `libcurl`, **ureq
2.x does not auto-detect `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` environment
variables.**  Without an explicit `ureq::Proxy` configured on the agent, ureq
attempts a direct connection and resolves the target hostname via
`getaddrinfo()`.

In the Claude sandbox, direct DNS lookups for arbitrary external hosts are
blocked.  `getaddrinfo()` therefore returns `EAI_NONAME` ("nodename nor servname
provided, or not known") for every external mirror — regardless of whether the
host appears in `allowedDomains`, and regardless of whether `HTTP_PROXY` is set
in the environment, because ureq is not consulting it.

`curl` works because `libcurl` reads `HTTP_PROXY` itself, sends the request to
the proxy in absolute-form HTTP, and never calls `getaddrinfo()` on the target.

## Fix Applied

`src/main.rs` reads `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` itself (in that
order, mirroring `ureq::Proxy::try_from_system`) and constructs a
`ureq::Proxy`, which is attached to the agent via `AgentBuilder::proxy(...)`.
With a proxy configured, ureq resolves only the **proxy host** — see ureq 2.12
`src/stream.rs` `connect_host`, where `netloc` is `proxy.server:proxy.port`
when a proxy is set, and `hostname:port` only when no proxy is configured.  The
target hostname is sent to the proxy in absolute-form (HTTP) or via `CONNECT`
(HTTPS) and is never passed to `getaddrinfo()` locally.

`NO_PROXY` is honoured with a small matcher (exact match plus dot-suffix
match), so loopback targets like `127.0.0.1` and `*.local` continue to bypass
the proxy.  This keeps the integration tests (which talk to mock servers on
`127.0.0.1`) working unchanged.

Parse failures fall through to the next env var, so a sandbox that exports
`ALL_PROXY=socks5h://…` (a scheme `ureq::Proxy::new` doesn't recognize)
gracefully falls back to `HTTPS_PROXY` or `HTTP_PROXY` instead of erroring.
The `socks-proxy` ureq feature is **not** required and is not enabled.

When none of the `*_PROXY` variables are set (unsandboxed environments),
behaviour is unchanged — ureq connects directly to the target.

## Key Observation: ureq's Native Proxy Support is Sufficient

The earlier ad-hoc workaround (commit `d90c3ec`, never landed on `main`)
hand-rolled HTTP/1.1 directly to the proxy socket.  That was unnecessary: ureq
2.x already speaks absolute-form HTTP and `CONNECT` correctly when given a
`Proxy`.  The only gap was that ureq doesn't auto-pick up `*_PROXY` env vars —
the application has to read them and call `Proxy::new()` itself.  Once that is
done, the rest of ureq's proxy machinery does the right thing.

## Settings Required for the Claude Sandbox

Beyond the `select-mirror` network issue, the investigation surfaced all the
filesystem and Mach-service allowlists needed for the `local-ubuntu` project to
function under the sandbox.  These are recorded in
`.claude/settings.json` in that repository.

### `allowRead` entries (within `denyRead: ["~/"]`)

| Path | Required by |
|---|---|
| `~/.claude/` | Claude Code itself |
| `~/Workspace/` | Working directory and project source |
| `~/.doppler/` | Doppler CLI config |
| `~/Library/Keychains/` | macOS Keychain (Doppler auth token) |
| `~/Library/Preferences/` | Keychain search-list preferences |
| `~/.gitconfig` | `git config user.name / user.email` at Justfile parse time |
| `~/.smartgit-ai` | Git extension config included by git |
| `~/.cargo/` | Rust toolchain (if building from source) |
| `~/.rustup/` | Rust toolchain (if building from source) |

### `allowWrite` entries

| Path | Required by |
|---|---|
| `/tmp` | General temp files |
| `~/Workspace/` | Cargo build output |
| `~/.cargo/` | Cargo registry / build cache |

### `allowMachLookup` entries

| Service | Required by |
|---|---|
| `com.apple.SecurityServer` | macOS Keychain access (Doppler auth token) |
| `com.apple.security.syspolicy` | Keychain / SecureTransport |
| `com.apple.trustd.agent` | TLS certificate verification |
| `com.apple.mDNSResponder` | DNS resolution via the system resolver |

> **Note** — `com.apple.mDNSResponder` is needed so that commands which use
> `getaddrinfo()` via the system resolver (curl, git, etc.) can resolve the
> proxy address (`localhost`) and any other hosts the sandbox proxy forwards.
> Direct DNS for arbitrary external hosts is still blocked at the proxy level.

### `allowLocalBinding`

```json
"sandbox": {
  "network": {
    "allowLocalBinding": true
  }
}
```

Required to run `select-mirror`'s integration tests (`tests/cli.rs`).  The
tests spin up mock HTTP servers via `TcpListener::bind("127.0.0.1:0")` and
without this flag the sandbox blocks the `bind()` syscall with
`EPERM` ("Operation not permitted").

The mock-server traffic correctly bypasses the sandbox HTTP proxy because
`127.0.0.1` is listed in `NO_PROXY` (and `select-mirror`'s
`host_matches_no_proxy` honours it), so the new `proxy_from_env` path is not
exercised for these tests — the requests go straight to the loopback listener.
