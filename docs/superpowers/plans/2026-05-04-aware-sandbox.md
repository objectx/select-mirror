# Aware-Sandbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the OpenSpec change `2026-05-04-aware-sandbox` (proxy-aware probing) — committing the already-written code, the OpenSpec artifacts, and the user-facing documentation in clean, reviewable commits, then mark the documentation milestone done in `tasks.md`.

**Architecture:** The implementation milestones 1–4 of the OpenSpec change are already complete in the working tree (`src/main.rs` reads `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` env vars, configures `ureq::Proxy`, honours `NO_PROXY`, falls through on parse failure; 35/35 tests + clippy clean; sandbox path verified end-to-end via the `local-ubuntu` consumer). This plan covers milestone 5 only — committing the working tree (split into one feat commit and two docs commits) and updating README.md / CLAUDE.md so the new env-var-driven behaviour is discoverable. The findings doc (`findings-about-claude-sandboxing.md`) is already coherent from earlier in the session and is included in the OpenSpec/docs commit unchanged.

**Tech Stack:** Rust 2021 (no new deps), Markdown, git conventional commits.

---

## Files

- Modify (already changed in working tree, needs committing): `src/main.rs`
- Untracked, needs adding+committing: `findings-about-claude-sandboxing.md`
- Untracked, needs adding+committing: `openspec/changes/2026-05-04-aware-sandbox/` (proposal.md, design.md, tasks.md, specs/aware-sandbox/spec.md, .openspec.yaml)
- Modify: `README.md` — add a "Sandboxes and proxies" section after "Caching"
- Modify: `CLAUDE.md` — update `probe` architecture bullet, add four new bullets for `build_agent`, `proxy_from_env`, `host_from_url`, `host_matches_no_proxy`
- Modify: `openspec/changes/2026-05-04-aware-sandbox/tasks.md` — mark milestone 5 complete after the docs are landed

Out of scope:
- `graphify-out/` (untracked, unrelated session output)
- `.selected-mirror.json` if present in CWD (gitignored)

---

### Task 1: Verify clean state and commit the implementation code

**Files:**
- Verify: `src/main.rs`, `Cargo.toml`, `Cargo.lock`
- Commit: `src/main.rs`

- [ ] **Step 1: Confirm tests pass and clippy is clean**

Run:

```bash
cargo test
```

Expected: all tests pass (the working tree currently shows 35 passed across the binary unit suite and `tests/cli.rs`).

Run:

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: no errors, no warnings.

If either fails, STOP. The plan assumes a green working tree. Re-investigate before proceeding.

- [ ] **Step 2: Inspect the staged-vs-unstaged delta to confirm scope**

Run:

```bash
git status --short
git diff --stat src/main.rs
```

Expected:
- `M src/main.rs` is modified.
- `?? findings-about-claude-sandboxing.md` is untracked.
- `?? openspec/changes/2026-05-04-aware-sandbox/` is untracked.
- `?? graphify-out/` may be present — leave it alone (unrelated).
- `Cargo.toml` and `Cargo.lock` should NOT be modified (no new deps were added).

If `Cargo.toml` or `Cargo.lock` shows changes, STOP and investigate — the design forbids new dependencies.

- [ ] **Step 3: Stage and commit the implementation**

Run:

```bash
git add src/main.rs
git commit -m "$(cat <<'EOF'
feat: honour HTTP_PROXY / NO_PROXY for sandbox-friendly probing

ureq 2.x does not auto-read *_PROXY env vars, so probes inside a
network-filtered sandbox (e.g. Claude Code's sandbox.enabled mode)
fail with EAI_NONAME on direct DNS. Read ALL_PROXY / HTTPS_PROXY /
HTTP_PROXY (and lowercase variants) ourselves, configure ureq's
native Proxy, and honour NO_PROXY with an exact-match-plus-dot-suffix
matcher so loopback targets keep working unchanged. Parse failures
fall through (sandbox sets ALL_PROXY=socks5h://… which ureq does not
recognize). No new dependencies; ureq's native HTTP/CONNECT proxy
support is sufficient once configured.
EOF
)"
```

- [ ] **Step 4: Confirm the commit landed cleanly**

Run:

```bash
git log -1 --stat
git status --short
```

Expected:
- `git log -1 --stat` shows the new commit touching only `src/main.rs`.
- `git status --short` no longer lists `M src/main.rs`; the two `??` entries (findings doc, openspec change) and any unrelated untracked files remain.

---

### Task 2: Commit the OpenSpec change and the findings doc together

**Files:**
- Add: `findings-about-claude-sandboxing.md`
- Add: `openspec/changes/2026-05-04-aware-sandbox/.openspec.yaml`
- Add: `openspec/changes/2026-05-04-aware-sandbox/proposal.md`
- Add: `openspec/changes/2026-05-04-aware-sandbox/design.md`
- Add: `openspec/changes/2026-05-04-aware-sandbox/tasks.md`
- Add: `openspec/changes/2026-05-04-aware-sandbox/specs/aware-sandbox/spec.md`

- [ ] **Step 1: Re-validate the OpenSpec change**

Run:

```bash
openspec list --json
openspec validate 2026-05-04-aware-sandbox
```

Expected:
- `openspec list --json` lists the change as `in-progress` with 4/5 tasks complete.
- `openspec validate 2026-05-04-aware-sandbox` prints `Change '2026-05-04-aware-sandbox' is valid`.

If validation fails, STOP. Re-read the schema at `openspec/schemas/superpowers-sdd/schema.yaml` and fix the artifact.

- [ ] **Step 2: Stage and commit**

Run:

```bash
git add findings-about-claude-sandboxing.md openspec/changes/2026-05-04-aware-sandbox
git commit -m "$(cat <<'EOF'
docs: record sandbox proxy support as openspec change

Adds findings-about-claude-sandboxing.md with the diagnosis and
required sandbox settings, and an openspec change
'2026-05-04-aware-sandbox' covering proposal, design, tasks, and
spec. The implementation in the previous commit satisfies
milestones 1–4; documentation (milestone 5) lands separately.
EOF
)"
```

- [ ] **Step 3: Confirm the commit**

Run:

```bash
git log -1 --stat
git status --short
```

Expected:
- The commit lists `findings-about-claude-sandboxing.md` plus all 5 OpenSpec files.
- `git status --short` no longer lists those entries.

---

### Task 3: Add a "Sandboxes and proxies" section to README.md

**Files:**
- Modify: `README.md` — insert a new section between the existing "Caching" section and "Build"

- [ ] **Step 1: Verify the insertion point**

Run:

```bash
grep -n '^## ' README.md
```

Expected output (line numbers may vary):

```
5:## Usage
31:## Example
53:## Caching
61:## Build
68:## Reference
```

The new section must be inserted between `## Caching` and `## Build`.

- [ ] **Step 2: Insert the new section**

Use the Edit tool to replace the text immediately preceding `## Build` with the new section followed by `## Build`.

`old_string`:

```
Use `--no-cache` to force a fresh probe while still updating the cache for the next run.

## Build
```

`new_string`:

```
Use `--no-cache` to force a fresh probe while still updating the cache for the next run.

## Sandboxes and proxies

`select-mirror` honours the standard proxy environment variables. When any of `ALL_PROXY`, `HTTPS_PROXY`, or `HTTP_PROXY` (or their lowercase equivalents) is set, probes are routed through the configured proxy. The first variable that parses as a `ureq` proxy URL wins; unparseable values (for example `socks5h://…`, a scheme `ureq 2.x` does not recognize) are skipped and the next variable is tried.

`NO_PROXY` is honoured with a minimal matcher: each comma-separated entry matches as either an exact hostname or a dot-suffix (`example.com` matches `api.example.com`). Leading `*.` or `.` on an entry is stripped before matching. Loopback entries such as `127.0.0.1`, `::1`, and `*.local` therefore bypass the proxy automatically when listed in `NO_PROXY`.

This makes `select-mirror` work transparently inside network-filtered sandboxes (e.g. Claude Code's `sandbox.enabled: true` mode) where outbound traffic is forced through a local HTTP proxy and direct DNS for arbitrary hosts is blocked. No flags or sandbox-specific configuration are required from the caller — the standard env-var contract is sufficient.

When none of the proxy variables are set, the tool connects directly as before.

## Build
```

- [ ] **Step 3: Verify the edit and that markdown structure stays clean**

Run:

```bash
grep -n '^## ' README.md
```

Expected output (line numbers will shift; the section list must contain `Sandboxes and proxies` between `Caching` and `Build`):

```
5:## Usage
31:## Example
53:## Caching
61:## Sandboxes and proxies
71:## Build
78:## Reference
```

(Exact line numbers can differ — what matters is the order: Caching → Sandboxes and proxies → Build → Reference.)

Spot-check the section with:

```bash
sed -n '/^## Sandboxes and proxies/,/^## Build/p' README.md
```

Expected: prints the new section followed by the `## Build` line. No stray blank-line clusters or broken backticks.

- [ ] **Step 4: Commit (deferred)**

Do NOT commit yet. Bundle README + CLAUDE.md in a single docs commit at the end of Task 4.

---

### Task 4: Update CLAUDE.md architecture bullets

**Files:**
- Modify: `CLAUDE.md` — replace the `probe` bullet and append four new bullets for the new helpers

- [ ] **Step 1: Confirm the current bullet text**

Run:

```bash
grep -n 'probe(mirror' CLAUDE.md
```

Expected: one match, currently:

```
25:- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — fires a ureq GET, returns elapsed seconds or `None` on failure/timeout
```

If the line is missing or differs in wording, STOP and read `CLAUDE.md` to determine the right anchor before continuing.

- [ ] **Step 2: Replace the `probe` bullet with the updated description plus four new bullets**

Use the Edit tool.

`old_string`:

```
- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — fires a ureq GET, returns elapsed seconds or `None` on failure/timeout
```

`new_string`:

```
- **`probe(mirror, probe_path, timeout_secs) -> Option<f64>`** — builds a per-URL `ureq::Agent` via `build_agent`, fires a GET, returns elapsed seconds or `None` on failure/timeout
- **`build_agent(url) -> ureq::Agent`** — constructs an agent, attaching a `ureq::Proxy` from env unless the URL's host matches `NO_PROXY`
- **`proxy_from_env() -> Option<ureq::Proxy>`** — reads `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` (and lowercase variants) in priority order; falls through on unparseable values
- **`host_from_url(url) -> Option<&str>`** — extracts the host from a URL, stripping port and bracketed IPv6
- **`host_matches_no_proxy(host) -> bool`** — exact-match plus dot-suffix match against `NO_PROXY` / `no_proxy`; leading `*.` and `.` on each entry are stripped before matching
```

- [ ] **Step 3: Verify the edit**

Run:

```bash
grep -n -E 'probe\(mirror|build_agent|proxy_from_env|host_from_url|host_matches_no_proxy' CLAUDE.md
```

Expected: 5 matches, one for each function bullet, in the order listed above.

- [ ] **Step 4: Re-read the architecture section as a sanity check**

Run:

```bash
sed -n '/^## Architecture/,/^The original shell/p' CLAUDE.md
```

Expected: bullets read top-to-bottom in the order Args → CacheEntry → CacheEntry::new → load_cache → save_cache → secs_to_ms → probe → build_agent → proxy_from_env → host_from_url → host_matches_no_proxy → find_best → main. The narrative still makes sense (no orphan references, no broken markdown).

- [ ] **Step 5: Commit README.md and CLAUDE.md together**

Run:

```bash
git add README.md CLAUDE.md
git commit -m "$(cat <<'EOF'
docs: document HTTP_PROXY/NO_PROXY support and architecture

Adds a 'Sandboxes and proxies' section to README documenting the
*_PROXY and NO_PROXY env-var contract, including the sandbox use
case. Updates CLAUDE.md's architecture bullets to mention the new
helpers (build_agent, proxy_from_env, host_from_url,
host_matches_no_proxy) and clarifies that probe now uses a
per-URL ureq::Agent.
EOF
)"
```

- [ ] **Step 6: Confirm the commit**

Run:

```bash
git log -1 --stat
git status --short
```

Expected:
- `git log -1 --stat` shows exactly two files: `README.md` and `CLAUDE.md`.
- `git status --short` shows no remaining tracked-file modifications. Untracked `graphify-out/` (and any cache files) may remain — they are out of scope.

---

### Task 5: Mark milestone 5 done in tasks.md and final verification

**Files:**
- Modify: `openspec/changes/2026-05-04-aware-sandbox/tasks.md`

- [ ] **Step 1: Flip the documentation milestone checkbox**

Use the Edit tool.

`old_string`:

```
- [ ] **Milestone 5: Documentation**
  Update `README.md` to document `*_PROXY` / `NO_PROXY` support and the sandbox-friendly behaviour. Update `CLAUDE.md`'s architecture summary to mention `build_agent`, `proxy_from_env`, and `host_matches_no_proxy`. Note the consumer-side `allowLocalBinding` requirement for running tests under a sandbox in `findings-about-claude-sandboxing.md`.
```

`new_string`:

```
- [x] **Milestone 5: Documentation**
  Update `README.md` to document `*_PROXY` / `NO_PROXY` support and the sandbox-friendly behaviour. Update `CLAUDE.md`'s architecture summary to mention `build_agent`, `proxy_from_env`, and `host_matches_no_proxy`. Note the consumer-side `allowLocalBinding` requirement for running tests under a sandbox in `findings-about-claude-sandboxing.md`.
```

- [ ] **Step 2: Re-validate the OpenSpec change**

Run:

```bash
openspec list --json
openspec validate 2026-05-04-aware-sandbox
```

Expected:
- `openspec list --json` shows the change with 5/5 tasks complete (`completedTasks: 5, totalTasks: 5`).
- `openspec validate 2026-05-04-aware-sandbox` prints `Change '2026-05-04-aware-sandbox' is valid`.

- [ ] **Step 3: Re-run the full local verification**

Run:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: tests pass, clippy clean.

- [ ] **Step 4: Commit the tasks.md update**

Run:

```bash
git add openspec/changes/2026-05-04-aware-sandbox/tasks.md
git commit -m "$(cat <<'EOF'
docs(openspec): mark aware-sandbox milestone 5 (Documentation) done
EOF
)"
```

- [ ] **Step 5: Final tree audit**

Run:

```bash
git log --oneline -5
git status --short
```

Expected:
- Five most recent commits (in reverse chronological order) include, in order:
  1. `docs(openspec): mark aware-sandbox milestone 5 (Documentation) done`
  2. `docs: document HTTP_PROXY/NO_PROXY support and architecture`
  3. `docs: record sandbox proxy support as openspec change`
  4. `feat: honour HTTP_PROXY / NO_PROXY for sandbox-friendly probing`
  5. The previous tip of `main` (e.g. `chore: archive cache-selected-mirror change; sync spec to main`).
- `git status --short` shows no tracked modifications. Only out-of-scope untracked files (e.g. `graphify-out/`) may remain.

If anything in the tree audit looks off, STOP — don't paper over surprise state with another commit.

- [ ] **Step 6: Hand off the change for archiving**

Surface to the human:

> All five milestones for `2026-05-04-aware-sandbox` are complete and committed. When you're ready to consolidate the spec into `openspec/specs/`, run `/opsx:archive 2026-05-04-aware-sandbox`. (Do not run archive automatically — that's a human-trigger.)

---

## Self-Review Notes

Spec coverage check (against `openspec/changes/2026-05-04-aware-sandbox/specs/aware-sandbox/spec.md`):

- `Probe honours HTTP-proxy environment variables` → satisfied by Task 1 commit (code already in working tree).
- `NO_PROXY bypasses the proxy` → satisfied by Task 1 commit.
- `Unparseable proxy values fall through` → satisfied by Task 1 commit.
- `No proxy configured uses direct connection` → satisfied by Task 1 commit (no behaviour change when env unset).

All four MUST/SHALL requirements are met by code already in the working tree; this plan covers the surrounding hygiene (commits + docs) only.

Placeholder scan: every step contains exact commands, exact file paths, and full edit blocks (no "TBD" / "appropriate" / "similar to"). Verification commands print expected output that the engineer can compare against.

Type/identifier consistency: function names referenced in CLAUDE.md (`build_agent`, `proxy_from_env`, `host_from_url`, `host_matches_no_proxy`) match `src/main.rs` exactly; `tasks.md` mentions the same trio plus the larger function set; commit messages reference the same names.
