<dev-flow>

# **MUST use DEV-FLOW to manage development workflow.**

## Commands
| Command | Purpose |
|---|---|
| `/init` | Initialize project |
| `/brainstorm` | Collaborative requirement exploration |
| `/prd` | Start PRD phase |
| `/spec` | Start SPEC phase |
| `/task` | Start TASK phase |
| `/issue` | Create an issue |
| `/devtest` | Routine dev testing |
| `/fix` | Auto-fix open issues |
| `/test` | Full test phase |
| `/status` | Report status |
| `/check` | Check doc sync |
| `/iterate` | Iterate delivery |
| `/mode` | Select dev mode |

## Discipline

### Claim before DEV
- Before writing code, run `dow claim <TASK-ID or ISSUE-ID>` to declare your work target. Claims expire after 5 minutes — re-claim if still working.
- After completing the task, run `dow claim --revoke` to release.

### DEV phase rules
- When hook output contains `[BLOCKED]`, stop all dev operations — only `/task`, `/issue`, `/iterate` allowed.
- Before starting a task, read the task file for full context (done_when, files, refs). If `refs` exists, read corresponding SPEC sections.
- Only do tasks listed in task/ — no more, no less.
- After completing a task, immediately run `/devtest`.
- After all tasks complete, auto-enter `/test`.

### Doc format
- When creating or writing .dev-doc files, use `dow doc <type> --json` to get format definition. Never write from memory.

### Role isolation
- PRD/SPEC/TASK/TEST phases run in independent agents. Each agent only receives minimal input for that phase.

{CODEX DEV FLOW Discipline}

</dev-flow>