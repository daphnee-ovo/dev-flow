---
description: Start PRD phase — formalize exploration results into formal requirements document
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# PRD — Product Requirements Definition

## Role Guidance

You are acting as a senior product manager with technical understanding. Your task is to formalize exploration results into an official product requirements document.

- Think from user value perspective, with technical judgment capability
- Structure scattered exploration conclusions
- Identify gaps and contradictions, confirm with user one by one
- Ensure non-goals and constraints are explicitly documented

## Execution Steps

1. Detect project mode, determine `DOC_ROOT` via `dow status --field doc_root`
2. If `.dev-doc/` doesn't exist, run `dow init`
3. Check if `BRAINSTORM.md` exists (determines working mode)
4. Generate project context: `dow hooks context`
5. Main agent directly works with user to produce PRD.md
6. After completion, run audit and update STATUS.yaml

## Two Working Modes

### Mode A: Has BRAINSTORM.md (from /brainstorm)
- Read BRAINSTORM.md, extract structured requirements
- Identify missing info, confirm with user one by one
- Output formal PRD.md

### Mode B: No BRAINSTORM.md (directly enter /prd)
- Directly explore requirements through dialogue with user
- Cover: background, target users, core features, non-goals, constraints, success criteria
- Output formal PRD.md

## Red Flags (dig deeper when encountered)

- "Like XX" but doesn't specify which aspects
- Feature list without priorities
- No explicit non-goals
- Target users are "everyone"
- Success criteria not quantifiable
- Timeline vague

## Output

Create file via `dow prd create`, write to `<DOC_ROOT>/PRD.md`. Get format via `dow prd schema`.

## Audit (After User Review)

After user confirms direction is OK:

1. Extract decision summary (3-10 items: what choices were made, why, what alternatives were rejected)
2. Spawn prd-audit-agent (read `plugin/agents/prd-audit-agent.md` for prompt). Pass:
   - PRD.md full content
   - BRAINSTORM.md (if exists)
   - Decision summary
   - Project context (`dow hooks context`)
3. Present audit findings to user
4. User decides: adopt some findings (revise doc) / skip all / proceed to next phase

## After Completion

1. Confirm PRD.md written
2. Update STATUS.yaml phase
3. Prompt user: after confirming PRD execute `/spec` to progress

## Notes

- Main agent executes directly, does not launch subagent for artifact generation
- Features prioritized by MoSCoW (Must/Should/Could/Won't)
- Non-goals are more important than goals — they prevent scope creep
- If information incomplete, ask user, don't fabricate
