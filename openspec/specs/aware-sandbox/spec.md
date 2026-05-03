# aware-sandbox Specification

## Purpose

Enable `select-mirror` to operate transparently inside network-filtered sandboxes (e.g. Claude Code's `sandbox.enabled: true` mode) by honouring the standard `*_PROXY` and `NO_PROXY` environment variables, without introducing new flags or dependencies. `ureq 2.x` — unlike `libcurl` — does not auto-detect these env vars; without an explicit `ureq::Proxy`, ureq attempts a direct connection and fails with `EAI_NONAME` because the sandbox blocks direct DNS for arbitrary external hosts. This capability covers the application-side configuration that bridges that gap.

## Requirements
### Requirement: Probe honours HTTP-proxy environment variables

When probing a mirror, the system SHALL consult the environment variables `ALL_PROXY`, `all_proxy`, `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy` in that order. The first variable whose value is non-empty AND parses as a `ureq::Proxy` SHALL be configured on the agent used for the probe. With a proxy configured, the probe SHALL NOT cause a local DNS lookup for the target hostname.

#### Scenario: HTTP_PROXY is set and the target is external

- **WHEN** `HTTP_PROXY=http://localhost:8888` is set, no `NO_PROXY` is configured, and the user runs `select-mirror http://archive.ubuntu.com/...`
- **THEN** the probe routes through `localhost:8888`
- **AND** the target hostname `archive.ubuntu.com` is not resolved by the local resolver
- **AND** the result is interpreted as success/failure of the underlying probe

#### Scenario: ALL_PROXY takes precedence over HTTP_PROXY

- **WHEN** `ALL_PROXY=http://proxy-a:1` and `HTTP_PROXY=http://proxy-b:2` are both set, and `ALL_PROXY` parses successfully
- **THEN** the probe is routed through `proxy-a:1`

#### Scenario: HTTPS_PROXY is honoured for HTTP targets

- **WHEN** only `HTTPS_PROXY=http://proxy:3128` is set and the user runs `select-mirror http://example.com/...`
- **THEN** the probe is routed through `proxy:3128`

### Requirement: NO_PROXY bypasses the proxy

When `NO_PROXY` (or `no_proxy`) is set, the system SHALL bypass the configured proxy for any target whose hostname matches a comma-separated entry in NO_PROXY. A match SHALL be either an exact equality OR a dot-suffix match (host ends with `.<entry>`). Leading `*.` and `.` on an entry SHALL be stripped before matching. When `NO_PROXY` matches, the probe SHALL behave as if no proxy variables were set.

#### Scenario: Loopback target bypasses proxy

- **WHEN** `HTTP_PROXY=http://corp-proxy:8080` and `NO_PROXY=localhost,127.0.0.1,::1,*.local` are both set, and the user runs `select-mirror http://127.0.0.1:9001/...`
- **THEN** the probe connects directly to `127.0.0.1:9001`
- **AND** does not contact `corp-proxy:8080`

#### Scenario: Dot-suffix match bypasses proxy

- **WHEN** `NO_PROXY=example.com` is set and the target is `http://api.example.com/...`
- **THEN** the probe bypasses the proxy because `api.example.com` ends with `.example.com`

#### Scenario: Wildcard pattern bypasses proxy

- **WHEN** `NO_PROXY=*.local` is set and the target is `http://mirror.local/...`
- **THEN** the probe bypasses the proxy because the leading `*.` is stripped to `local` and `mirror.local` ends with `.local`

#### Scenario: Non-matching host still uses proxy

- **WHEN** `NO_PROXY=example.com` is set and the target is `http://evil-example.com/...`
- **THEN** the probe routes through the proxy (substring matches that do not align on a `.` boundary do not match)

### Requirement: Unparseable proxy values fall through

If a proxy value parses as the unsupported scheme or otherwise fails `ureq::Proxy::new`, the system SHALL silently fall through to the next environment variable in priority order. A parse failure SHALL NOT cause the probe to fail or the program to exit non-zero.

#### Scenario: ALL_PROXY uses an unsupported scheme

- **WHEN** `ALL_PROXY=socks5h://localhost:1080` is set and `HTTP_PROXY=http://localhost:8888` is also set
- **THEN** `ALL_PROXY` is rejected by `ureq::Proxy::new` (the `socks5h://` scheme is unrecognized)
- **AND** the system falls through and uses `HTTP_PROXY=http://localhost:8888` instead

#### Scenario: Empty proxy value is skipped

- **WHEN** `ALL_PROXY=""` is set and `HTTP_PROXY=http://localhost:8888` is set
- **THEN** the empty `ALL_PROXY` is treated as unset and the probe uses `HTTP_PROXY`

### Requirement: No proxy configured uses direct connection

When none of the recognized proxy environment variables are set (or all of them are empty/unparseable), the probe SHALL connect directly to the target as it did before this change. Behaviour outside a sandbox MUST be unchanged.

#### Scenario: No proxy environment variables set

- **WHEN** none of `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` (and lowercase variants) are set, and the user runs `select-mirror http://archive.ubuntu.com/...`
- **THEN** the probe connects directly to `archive.ubuntu.com`
- **AND** the elapsed time and pass/fail outcome match the pre-change behaviour

