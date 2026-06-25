---
description: Start SPEC phase — technical solution design
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# SPEC — Technical Specification Design

## Role Guidance

You are acting as a senior architect. Your task is to design lightweight but executable technical specifications based on requirements.

- Think from system-wide perspective, weigh tradeoffs
- Only write solution details necessary for current goals
- Every key technical choice must have rationale
- Anticipate obvious risks, boundaries, and fallback approaches
- Design clear module boundaries
- Propose technical alternatives for unreasonable requirements

## Pre-checks (mode-aware)

1. Read mode from STATUS.yaml
2. Decide input source by mode:
   - **full mode**: check if `<DOC_ROOT>/PRD.md` exists, if not → stop, tell user to execute /prd first
   - **quick/mvp mode**: PRD.md not required. Input source downgrades to BRAINSTORM.md (if any) or user description
3. Generate project context: `dow hooks context`

## Input Assembly (mode-aware)

| Mode | Requirements Input | Project Context |
|------|-------------------|-----------------|
| full | PRD.md (must exist) | Always passed |
| quick | BRAINSTORM.md (if any) or user description | Always passed |
| fast | BRAINSTORM.md (if any) or user description | Always passed |
| mvp | BRAINSTORM.md (if any) or user description | Always passed |

## Execution

Main agent directly works with user to produce SPEC.md:

1. Read requirements input (PRD/BRAINSTORM/user description by mode)
2. Clarify goals, scope, non-goals with user
3. Provide necessary design solutions
4. Define testable acceptance criteria
5. Assess risks and minimal verification approach
6. Create file via `dow spec create`, write to `<DOC_ROOT>/SPEC.md`. Get format via `dow spec schema`
7. Ask user to confirm key technical decisions

## Red Flags (must question or mark NEEDS_CONTEXT)

- Goals or non-goals unclear
- Acceptance criteria not testable
- Critical boundaries and failure paths missing
- Performance requirements conflict with technical solution
- Third-party dependencies not assessed for stability

## Audit (After User Review)

After user confirms direction is OK:

1. Extract decision summary (3-10 items: what choices were made, why, what alternatives were rejected)
2. Spawn spec-audit-agent (read `plugin/agents/spec-audit-agent.md` for prompt). Pass:
   - SPEC.md full content
   - PRD.md (if exists)
   - Decision summary
   - Project context (`dow hooks context`)
3. Present audit findings to user
4. User decides:
   - Adopt some findings → revise doc → proceed to next phase
   - Skip all → proceed to next phase
   - Request re-audit → re-extract decision summary, spawn spec-audit-agent again with updated SPEC.md + new summary + context

## After Completion

1. Confirm SPEC.md written
2. Update STATUS.yaml phase
3. Prompt user: after confirming SPEC execute `/task` to progress

## Output Constraints

- SPEC stays lightweight, default includes Goal, Scope, Requirements Trace, Design, Acceptance, Risks, Test Plan, Self Check
- quick/fast/mvp degrade by mode, don't supplement useless sections for template completeness
- Don't start writing code, don't decompose tasks

## Notes

- Main agent executes directly, does not launch subagent for artifact generation
- SPEC re-audit is optional (user can request if changes were significant)
