# bump-ureq-to-v3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the `ureq` HTTP client from v2 to v3, deleting ~95 lines of manual proxy/NO_PROXY helper code that ureq 3 now handles natively.

**Architecture:** Single file change (`src/main.rs`) — rewrite `probe()` to use ureq 3's `Agent::config_builder().timeout_global()` API, then delete `build_agent`, `proxy_from_env`, `host_from_url`, `host_matches_no_proxy`, and their 5 unit tests. Proxy and NO_PROXY env-var handling is fully delegated to ureq 3 internals; no behaviour change for callers.

**Tech Stack:** Rust, ureq 3, cargo

---

### Task 1: Bump ureq version and confirm the build breaks

**Files:**
- Modify: `Cargo.toml:12`

- [ ] **Step 1: Bump ureq in Cargo.toml**

Change line 12 of `Cargo.toml`:
```toml
ureq = "3"
```

- [ ] **Step 2: Run cargo build and observe expected errors**

```bash
cargo build 2>&1 | head -40
```

Expected: errors referencing `AgentBuilder`, `ureq::Proxy::new`, and `.timeout(...)` on a request builder. These confirm the ureq 2 API is gone and that the next task must fix them.

---

### Task 2: Rewrite probe() with the ureq 3 API and delete build_agent()

**Files:**
- Modify: `src/main.rs:103-129`

- [ ] **Step 1: Replace probe() and delete build_agent() (lines 103–129)**

Remove the existing `probe` function and the entire `build_agent` block below it (including the comment block starting at line 115). Replace with:

```rust
fn probe(mirror: &str, probe_path: &str, timeout_secs: u64) -> Option<f64> {
    let url = format!("{}{}", mirror, probe_path);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .build()
        .into();
    let start = Instant::now();
    agent
        .get(&url)
        .call()
        .ok()
        .map(|_| start.elapsed().as_secs_f64())
}
```

What changed:
- `ureq::AgentBuilder::new()` → `ureq::Agent::config_builder().build().into()`
- `.timeout(duration)` was on the request; now `.timeout_global(Some(duration))` is on the agent config
- No proxy wiring — ureq 3 reads `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` from env automatically
- `build_agent()` is gone (its only job was manual proxy wiring)

- [ ] **Step 2: Run cargo build**

```bash
cargo build 2>&1
```

Expected: the `probe`-related errors are gone. Remaining errors (if any) are about `proxy_from_env`, `host_from_url`, `host_matches_no_proxy` being unused — those get deleted in the next task.

---

### Task 3: Delete proxy_from_env(), host_from_url(), host_matches_no_proxy()

**Files:**
- Modify: `src/main.rs` — delete three functions (~50 lines)

These functions exist immediately after `probe()`. Their line numbers have shifted after Task 2 but their signatures are unique.

- [ ] **Step 1: Delete proxy_from_env()**

Find and delete the entire function starting with:
```rust
fn proxy_from_env() -> Option<ureq::Proxy> {
```
Delete everything through the closing `}`.

- [ ] **Step 2: Delete host_from_url()**

Find and delete the entire function starting with:
```rust
fn host_from_url(url: &str) -> Option<&str> {
```
Delete everything through the closing `}`.

- [ ] **Step 3: Delete host_matches_no_proxy()**

Find and delete the entire function starting with:
```rust
fn host_matches_no_proxy(host: &str) -> bool {
```
Delete everything through the closing `}`.

- [ ] **Step 4: Run cargo build**

```bash
cargo build 2>&1
```

Expected: clean build, or only errors in the `#[cfg(test)]` block from the unit tests that still reference the deleted functions. No errors in production code.

---

### Task 4: Delete the 5 unit tests for removed helpers, verify, and commit

**Files:**
- Modify: `src/main.rs` — delete 5 test functions inside `#[cfg(test)]`

- [ ] **Step 1: Delete the 5 unit tests**

In the `#[cfg(test)]` module, find and delete each of these `#[test]` + `fn` blocks in full:

1. `fn host_from_url_extracts_simple_host()`
2. `fn host_from_url_strips_port()`
3. `fn host_from_url_handles_bracketed_ipv6()`
4. `fn host_from_url_returns_none_without_scheme()`
5. `fn no_proxy_matcher_exact_and_suffix()`

For each: delete the `#[test]` line, the `fn <name>() {` line, the body, and the closing `}`.

- [ ] **Step 2: Run cargo test**

```bash
cargo test 2>&1
```

Expected: all tests pass, zero warnings.

- [ ] **Step 3: Run integration tests specifically**

```bash
cargo test --test cli 2>&1
```

Expected: all integration tests in `tests/cli.rs` pass. These cover the real probe behaviour including timeout handling, confirming no regression from the ureq API change.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "$(cat <<'EOF'
chore: bump ureq to v3, remove manual proxy helpers

ureq 3 reads *_PROXY and NO_PROXY environment variables automatically.
Deleted build_agent, proxy_from_env, host_from_url, host_matches_no_proxy
and their 5 unit tests (~95 lines). probe() now uses
Agent::config_builder().timeout_global() for per-probe timeouts.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```
