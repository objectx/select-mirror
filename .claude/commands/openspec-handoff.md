# OpenSpec → Superpowers Handoff

Hand off the approved OpenSpec change to Superpowers for implementation.

## Instructions

1. Find the active OpenSpec change in `openspec/changes/` (not archived).
   If multiple active changes exist, ask which one to implement.
   If no active change exists, tell the human to run `/opsx:propose` first.

2. Invoke the `openspec-handoff` skill. It will:
   - Read proposal.md, design.md, and specs/ from the active change
   - Confirm understanding with the human
   - Produce a Superpowers implementation plan via `superpowers:writing-plans`
   - Execute via `superpowers:subagent-driven-development` with TDD and code review

3. After implementation, remind the human to run `/opsx:archive`.
