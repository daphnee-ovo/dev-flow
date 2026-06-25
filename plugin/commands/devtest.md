---
description: Routine dev testing — task-level minimal loop
allowed-tools: Agent, Bash, Read, Write, Edit
---

# DEV-TEST — Routine Testing

devtest only does lightweight loop, not large controller.

## Execution Method

```bash
dow test --task <TASK-ID>
```

Mode controlled by exec_mode field in STATUS.yaml (`dow status set --exec-mode continuous` or `step`),
devtest reads and executes in corresponding mode automatically.

## Pre-checks

1. Current phase must be `DEV`.
2. There must be tasks marked `[x]` in `task/`.
3. Must execute devtest after each task completion.

## Three Outcomes

| Outcome | Behavior |
|---------|----------|
| `PASS` | Keep task checked; if all tasks done, suggest `/test` |
| `FAIL` | Uncheck current task, create issue file via `dow issue create --source devtest`, stop progression |
| `NEEDS_CONTEXT` | Keep task checked, don't continue progression, require additional information |

## Agent Verification Requirements

If independent agent verification needed, only pass minimal context:

| Allowed Input | Prohibited Input |
|---------------|------------------|
| Current task title, refs, files, done_when | Other unrelated tasks |
| Related acceptance items in SPEC | PRD.md |
| Project context `dow hooks context` | Dev phase conversation history |

Verification focus:

- Judge pass/fail against `done_when` and related SPEC acceptance.
- When testing needed, prioritize running tests listed in task `files.test`.
- Don't introduce default TDD, don't expand requirement scope.
- Prohibited to write to system temp directories; project-internal `tmp` and `temp` both allowed, prioritize existing directories, new projects default to `tmp`.

## Output Protocol

Subagent or main agent must output at end:

```text
STATUS: PASS | FAIL | NEEDS_CONTEXT
DETAIL: <explanation>
```

If no explicit status, treat as `NEEDS_CONTEXT`, don't auto-progress.
