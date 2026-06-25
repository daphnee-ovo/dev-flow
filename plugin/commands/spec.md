---
description: Start SPEC phase — technical solution design
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# SPEC — Technical Specification Design

## Pre-checks (mode-aware)

1. Read mode from STATUS.yaml
2. Decide input source by mode:
   - **full mode**: check if `<DOC_ROOT>/PRD.md` exists, if not → stop, tell user to execute /prd first
   - **quick/mvp mode**: PRD.md not required to exist. Input source downgrades to BRAINSTORM.md (if any) or user description
3. Generate project context: `dow inbox context`

## Input Assembly (mode-aware)

| Mode | Requirements Input | Project Context |
|------|-------------------|-----------------|
| full | PRD.md (must exist) | Always passed |
| quick | BRAINSTORM.md (if any) or user description | Always passed |
| mvp | BRAINSTORM.md (if any) or user description | Always passed |

## Agent Dispatch (Isolation Template)

**Must launch independent subagent. Dispatch by current runtime: Claude Code uses `Agent`, Codex uses `spawn_agent`. Subagent prompt must use following content:**

```
description: "SPEC agent - Technical specification design"
prompt: `<read complete content of agents/spec-agent.md>

## Input Documents

### Requirements Source
<pass by mode: PRD.md complete content / BRAINSTORM.md content / user description>

### Project Context
<execute dow inbox context output, paste as-is>

## Output Path

Create file via `dow spec create`, write to `<DOC_ROOT>/SPEC.md`. Get format via `dow spec schema`.

## Prohibited

- Don't read unrelated historical files
- Don't reference PRD/BRAINSTORM discussion process (you can't see it, and don't need to)
- Don't decompose tasks (that's TASK phase's job)
- Don't start writing code`
```

## Input Isolation Rules

| Allowed Input | Prohibited Input |
|---------------|------------------|
| agents/spec-agent.md content | PRD/BRAINSTORM phase conversation discussion process |
| PRD.md / BRAINSTORM.md content (by mode) | User-agent interaction history |
| Project context (context.sh output) | TASK.md / TEST.md |
| DOC_ROOT path | Unrelated historical records |

## Isolation Boundary Explanation

What's isolated is **discussion process**, not **project current state**. SPEC agent needs to understand project current structure (directories, tech stack, existing modules) to make reasonable architecture decisions, but shouldn't see rejected approaches from requirements discussion.

## After Completion

1. Confirm SPEC.md written
2. Update STATUS.yaml: current phase → SPEC
3. Prompt user: after confirming SPEC execute `/task` to progress

## Output Constraints

- SPEC stays lightweight, default includes Goal, Scope, Requirements Trace, Design, Acceptance, Risks, Test Plan, Self Check.
- quick/fast/mvp degrade by mode, don't supplement useless sections for template completeness.
- Write Change directly in Requirements Trace Notes, don't separately create Change Delta.
- Don't start writing code, don't decompose tasks.
