## MODIFIED Requirements

### Requirement: Probe honours HTTP-proxy environment variables
When probing a mirror, the system SHALL consult the environment variables `ALL_PROXY`, `all_proxy`, `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy` in that order. The first variable whose value is non-empty and parseable as a proxy URI SHALL be used for that probe. With a proxy configured, the probe SHALL NOT cause a local DNS lookup for the target hostname.

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

### Requirement: Unparseable proxy values fall through
If a proxy environment variable contains a malformed URI that cannot be parsed, the system SHALL silently fall through to the next environment variable in priority order. A parse failure SHALL NOT cause the probe to fail or the program to exit non-zero.

#### Scenario: ALL_PROXY is malformed

- **WHEN** `ALL_PROXY=not-a-uri` is set and `HTTP_PROXY=http://localhost:8888` is also set
- **THEN** `ALL_PROXY` fails to parse as a proxy URI
- **AND** the system falls through and uses `HTTP_PROXY=http://localhost:8888` instead

#### Scenario: Empty proxy value is skipped

- **WHEN** `ALL_PROXY=""` is set and `HTTP_PROXY=http://localhost:8888` is set
- **THEN** the empty `ALL_PROXY` is treated as unset and the probe uses `HTTP_PROXY`
