---
description: Start complete TEST phase — project-level comprehensive verification
allowed-tools: Agent, Bash, Read, Write, Edit
---

# TEST — Project Testing (comprehensive verification)

## Pre-checks (blocking)

1. Read all active task files under `task/` directory, count incomplete tasks
2. If has incomplete tasks → **stop, tell user to complete all tasks first**, don't continue
3. Check if `<DOC_ROOT>/issue/` has unclosed issues → remind to fix first

## Phase Switch

**Before launching agent**, immediately update STATUS.yaml to TEST.

## Input Assembly

1. Generate project context: `dow inbox context`
2. Collect all task file content (including done_task_*) as verification scope
3. Read SPEC.md as verification standard

## Agent Dispatch (Isolation Template)

**Launch brand new independent TEST agent (full version), absolutely don't reuse dev context. Dispatch by current runtime: Claude Code uses `Agent`, Codex uses `spawn_agent`. Subagent prompt must use following content:**

```
description: "Project TEST - Comprehensive verification"
prompt: `<read complete content of agents/test-agent.md>

## Input Documents

### SPEC.md (verification standard)
<SPEC.md complete content, paste as-is>

### Task Files (verification scope)
<all task file content under task/ directory (including done_task_*), paste as-is>

### Project Context
<execute dow inbox context output, paste as-is>

## Output Paths

- Test code: tests/ (test_<feature>.<ext>)
- Test report: create `<DOC_ROOT>/TEST.md` via `dow doc test`
- Issue files: create via `dow doc issue --source test`, get format via `dow doc issue --json`

## Prohibited

- Don't view git log or commit history
- Don't reference any conversation from dev process
- Don't report issue because features not required by SPEC are missing
- Don't report issue for content marked "non-goal" in TASK
- Don't trust "developer said tested" — verify yourself`
```

## Input Isolation Rules

| Allowed Input | Prohibited Input |
|---------------|------------------|
| agents/test-agent.md content | PRD.md |
| SPEC.md complete content (original) | Any conversation history from dev phase |
| All task file content under task/ directory (including done_task_*) | git log / commit messages |
| Project context (context.sh output) | Routine TEST result history |
| Current date (for issue filename) | |

## Why Strict Isolation

TEST agent must be **completely independent from dev agent**. Developers unconsciously avoid weak points in their own code. Only brand new perspective, only reading docs not process, independent testing can find true blind spots.

Project context helps TEST agent quickly understand how to run project and existing test structure, doesn't need to spend lots of tokens exploring.

## Result Handling

- **All pass** → execute /iterate for delivery
- **Found problems** → issue files written, STATUS switches back to DEV, fix then /test again
