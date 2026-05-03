# Implementation Milestones

## Milestones

- [x] **Milestone 1: Proxy-aware probe**
  Read `*_PROXY` env vars in priority order (`ALL_PROXY` → `HTTPS_PROXY` → `HTTP_PROXY`, plus lowercase variants), construct a `ureq::Proxy`, and attach it to the agent used by `probe()`. Fall through on parse failures. Satisfies the proxy-honouring and parse-failure-fall-through requirements.

- [x] **Milestone 2: NO_PROXY bypass**
  Implement a minimal `NO_PROXY` matcher (exact match + dot-suffix; strip leading `*.` and `.` on each entry) and short-circuit proxy selection when the target hostname matches. Satisfies the NO_PROXY bypass requirement and keeps integration tests against `127.0.0.1` working unchanged.

- [x] **Milestone 3: Test coverage**
  Add unit tests for `host_from_url` (simple host, port-stripping, bracketed IPv6, missing scheme) and `host_matches_no_proxy` (exact, dot-suffix, wildcard, non-match). Verify that the existing integration suite still passes without modification.

- [x] **Milestone 4: Sandbox verification**
  Confirm end-to-end behaviour by running `select-mirror` from inside a Claude Code-sandboxed consumer (`local-ubuntu`). Verified working.

- [x] **Milestone 5: Documentation**
  Update `README.md` to document `*_PROXY` / `NO_PROXY` support and the sandbox-friendly behaviour. Update `CLAUDE.md`'s architecture summary to mention `build_agent`, `proxy_from_env`, and `host_matches_no_proxy`. Note the consumer-side `allowLocalBinding` requirement for running tests under a sandbox in `findings-about-claude-sandboxing.md`.
