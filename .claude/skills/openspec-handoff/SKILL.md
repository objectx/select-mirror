---
name: openspec-handoff
description: >-
  Use when an OpenSpec change exists in openspec/changes/ with an approved
  proposal.md and design.md — routes directly to writing-plans, skipping
  brainstorming. Bridges OpenSpec specifications into Superpowers execution.
---

# OpenSpec → Superpowers Handoff

## Overview

This skill bridges OpenSpec's specification phase into Superpowers'
implementation pipeline. It replaces the brainstorming skill for changes
that already have approved OpenSpec artifacts.

For full workflow context, read `docs/workflow.md` in the project root.

## When This Skill Applies

- An `openspec/changes/<n>/` directory exists
- It contains `proposal.md` AND `design.md`
- The change is NOT in `openspec/changes/archive/`

If these conditions are met, this skill takes priority over
`superpowers:brainstorming`.

## When This Skill Does NOT Apply

- No active OpenSpec change → use `superpowers:brainstorming` normally
- Quick spikes or exploratory work → use `superpowers:brainstorming`
- Debugging sessions → use `superpowers:systematic-debugging`

## The Handoff Process

### Step 1: Read the OpenSpec Artifacts

Read these files (use the Read tool, do not preload all of them):

1. `openspec/changes/<n>/proposal.md` — understand the "why"
2. `openspec/changes/<n>/design.md` — understand the "how"
3. `openspec/changes/<n>/specs/*.md` — understand the constraints
   (MUST/SHALL/SHOULD requirements and Given/When/Then scenarios)
4. `openspec/changes/<n>/tasks.md` — treat as high-level milestones only

### Step 2: Confirm Understanding

Present a brief summary to the human:

> I've read the OpenSpec artifacts for `<change-name>`.
>
> **Goal**: [1-2 sentences from proposal.md]
> **Approach**: [1-2 sentences from design.md]
> **Key constraints**: [list MUST/SHALL requirements]
>
> I'll now create a Superpowers implementation plan based on this design.
> Ready to proceed?

Wait for human confirmation.

### Step 3: Activate writing-plans

Invoke `superpowers:writing-plans` with the following context:

- The **design document** is `openspec/changes/<n>/design.md`
- The **spec constraints** are in `openspec/changes/<n>/specs/`
- The plan MUST satisfy all MUST/SHALL requirements from the specs
- The plan SHOULD satisfy all SHOULD requirements
- The plan MAY satisfy MAY requirements if low cost

The plan is saved to:
`docs/superpowers/plans/YYYY-MM-DD-<change-name>.md`

### Step 4: Proceed to Implementation

After the plan is written and approved, follow the standard
Superpowers execution pipeline:

1. `superpowers:using-git-worktrees` — branch isolation
2. `superpowers:subagent-driven-development` — task dispatch
3. `superpowers:test-driven-development` — mandatory TDD
4. `superpowers:requesting-code-review` — between batches
5. `superpowers:finishing-a-development-branch` — merge/PR

### Step 5: Archive the OpenSpec Change

After the branch is merged, remind the human:

> Implementation complete. Run `/opsx:archive <change-name>` to
> consolidate specs into `openspec/specs/`.

Do NOT run `/opsx:archive` automatically — the human decides when
the change is ready to archive.

## Rationalization Prevention

**"The OpenSpec design is vague, let me brainstorm instead."**
→ No. If design.md exists and was approved, it is the design. If you
believe it's insufficient, ask the human to refine it via OpenSpec, not
by starting a Superpowers brainstorm.

**"The OpenSpec tasks.md is already a plan, I don't need writing-plans."**
→ No. OpenSpec tasks.md contains high-level milestones (enforced by the
superpowers-sdd schema). Superpowers writing-plans produces the granular,
file-level, test-first plan that subagent-driven-development requires.
Always produce a Superpowers plan.

**"I'll just skip to coding, the spec is clear enough."**
→ No. The pipeline is: OpenSpec spec → Superpowers plan → Superpowers
execution. No shortcuts. The plan is where file paths, test strategies,
and verification steps are defined. Without it, subagents will improvise.

**"Let me use /opsx:apply instead of Superpowers execution."**
→ No. `/opsx:apply` does not enforce TDD, does not use subagents, and
does not perform code review. Superpowers' execution pipeline exists
for quality enforcement. Always use it.

## Quick Reference
