---
description: Start TASK phase — task decomposition
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# TASK — Task Decomposition

## Pre-checks (mode-aware)

1. Read development mode from STATUS.yaml
2. Decide input source by mode:
   - **full/quick mode**: check if `<DOC_ROOT>/SPEC.md` exists, if not stop, tell user to execute /spec first
   - **fast mode**: SPEC.md not required, use user description + project context as input
3. Generate project context: `dow inbox context`

## Input Assembly (mode-aware)

| Mode | Solution Input | Project Context |
|------|---------------|-----------------|
| full/quick | SPEC.md (must exist) | Always passed |
| fast | User description (no need for SPEC.md) | Always passed |

## Agent Dispatch (Isolation Template)

**Must launch independent subagent. Dispatch by current runtime: Claude Code uses `Agent`, Codex uses `spawn_agent`. Subagent prompt must use following content:**

```
description: "TASK agent - Task decomposition"
prompt: `<read complete content of agents/task-agent.md>

## Input Documents

### Technical Solution
<pass by mode: SPEC.md complete content / user description>

### Project Context
<execute dow inbox context output, paste as-is>

## Output Path

Create file via `dow doc task -n <task_count>` (auto-handles directory creation and sequence increment), write to `<DOC_ROOT>/task/task_<YYYY-MM-DD>_<seq>.md`.

## Output Format

Execute `dow doc task --json` to get structured format definition, append to agent prompt.

## Prohibited

- Don't read PRD.md (you don't need to know "why do it", only need to know "how to do it")
- Don't reference SPEC discussion process
- Don't start writing code
- Don't design new architecture solutions
- Don't add model/steps/verification/docs fields`
```

## Input Isolation Rules

| Allowed Input | Prohibited Input |
|---------------|------------------|
| agents/task-agent.md content | PRD.md |
| SPEC.md complete content (by mode) | PRD/SPEC phase conversation history |
| Project context (context.sh output) | Unrelated historical records |
| DOC_ROOT path | |

## Isolation Boundary Explanation

What's isolated is **discussion process**, not **project current state**. TASK agent needs to understand project directory structure and existing modules to reasonably assess task granularity, but shouldn't see how SPEC was discussed out.

## After Completion

1. Confirm task files written to `<DOC_ROOT>/task/`
2. Update STATUS.yaml: current phase → TASK
3. Prompt user: after confirming task list, STATUS will switch to DEV, start development
