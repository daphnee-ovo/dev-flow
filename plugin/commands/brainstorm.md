---
description: Pre-implementation collaborative requirement exploration and design (conversational brainstorm)
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# BRAINSTORM — Turn Ideas into Design

Transform vague ideas into complete design through natural conversation, reaching consensus before implementation.

<HARD-GATE>
No implementation work may begin before presenting the design and obtaining explicit user agreement. No implementation-related skills may be invoked and no business code written. This applies to all projects, no matter how "simple" they seem.
</HARD-GATE>

## Anti-Pattern: "This is too simple, doesn't need design"

Every project needs this process. A utility function, a config change, a single-page app — all need it. "Simple" projects are exactly where unexamined assumptions cause most rework. Design can be short (simple projects just a few sentences), but must be presented and approved.

## Checklist (must complete in order)

1. **Explore project context** — check files, docs, recent commits
2. **Assess scope** — if too large, decompose into sub-projects first
3. **Ask clarifying questions one by one** — one question per message, understand purpose/constraints/success criteria
4. **Propose 2-3 approaches** — with trade-offs and recommendation
5. **Present design in segments** — segment by complexity, confirm each segment before continuing
6. **Write design doc** — create via `dow brainstorm create`, write to `<DOC_ROOT>/BRAINSTORM.md`
7. **Design self-check** — check for placeholders, contradictions, ambiguities, scope
8. **User review** — ask user to review doc, only continue after confirmation
9. **Transition to next phase** — suggest entering `/prd` or `/spec`

## Exploration Phase

**Understand the idea:**

- First check current project state (files, docs, git history)
- Assess scope: if description contains multiple independent subsystems (e.g., "build a platform with chat, storage, billing"), **point out immediately**, help decompose into sub-projects. Don't dive into details on overly broad scope. Each sub-project goes through brainstorm → prd → spec → implementation cycle independently.
- For reasonably scoped projects, clarify with questions one by one

**Questioning principles:**

- **One question per message** — don't throw multiple questions at once
- **Prioritize multiple choice** — giving 2-4 options is easier to answer than open-ended
- **Focus on key dimensions** — purpose, user groups, constraints, success criteria, technical preferences
- If a topic needs deep exploration, break into multiple questions progressing step by step

## Solution Exploration

**Propose approaches:**

- Propose 2-3 different approaches, explain trade-offs for each
- Present conversationally, start with recommended approach and rationale
- Don't write lengthy report-style documents

## Present Design

**Segment presentation:**

- Adjust segment length by complexity: simple ones a few sentences, complex ones detailed explanation
- After each segment ask user "is this part OK", continue to next segment after confirmation
- Cover dimensions: architecture, components, data flow, error handling, test strategy

**Design isolation principle:**

- Break system into small units, each with **one clear responsibility**
- Units communicate through **clearly defined interfaces**
- Each unit can be **independently understood and tested**
- For each unit, can answer: what does it do, how to use it, what does it depend on
- Can you understand unit function without reading internal implementation? Can you change internal implementation without breaking callers? If not, boundaries need redesign
- Small, focused units also easier to reason about and modify — when files grow large, usually means they bear too many responsibilities

**Working in existing codebase:**

- Before proposing changes, explore existing structure first. Follow existing patterns.
- If existing code has problems affecting this work (files too large, boundaries unclear, responsibilities tangled), include targeted improvements in design — like a good engineer improving code in their work area.
- Don't propose unrelated refactoring. Stay focused.

## Write Design Doc

After confirmation, create file via `dow brainstorm create` (auto-writes to `<DOC_ROOT>/BRAINSTORM.md`), get format via `dow brainstorm schema`.

## Design Self-Check

After writing doc, review with fresh perspective:

1. **Placeholder scan** — any TBD, TODO, incomplete parts, vague requirements? Fix them.
2. **Internal consistency** — contradictions between parts? Architecture and feature descriptions match?
3. **Scope check** — focused enough to enter single PRD/SPEC? Or needs further decomposition?
4. **Ambiguity check** — any requirement that can be understood two ways? If so, pick one and write explicitly.

Fix problems directly, no need to restart process.

## User Review Gate

After self-check passes, ask user to review:

> "Design doc written to `<DOC_ROOT>/BRAINSTORM.md`. Please review, tell me if anything needs modification. We'll proceed to next step after confirmation."

Wait for user response. If modification requested, revise then re-run self-check. Only transition after user confirms.

## Audit (After User Review)

After user confirms direction is OK:

1. Extract decision summary (3-10 items: what choices were made, why, what alternatives were rejected)
2. Spawn brainstorm-audit-agent (read `plugin/agents/brainstorm-audit-agent.md` for prompt). Pass:
   - BRAINSTORM.md full content
   - Decision summary
   - Project context (`dow hooks context`)
3. Present audit findings to user
4. User decides: adopt some findings (revise doc) / skip all / proceed to next phase

## Transition to Next Phase

- Requirements already clear, can directly produce technical spec → suggest `/spec`
- Still need more formal requirements doc to define scope → suggest `/prd`
- User decides

## Core Principles

- **YAGNI** — cut unnecessary features, ask for every design "is this really needed?"
- **One at a time** — don't drown user with questions
- **Progressive validation** — confirm each step before moving forward, don't assume
- **Flexible exit** — if user says "enough, just start", respect judgment, write current state then transition

## Notes

- Pure conversational process, main agent executes directly, doesn't launch subagent
- BRAINSTORM.md not archived during `/iterate` (kept as persistent project reference)
- If .dev-doc/ doesn't exist, brainstorm will create it
