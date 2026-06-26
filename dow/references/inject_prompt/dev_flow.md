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
- Before starting a task, use `dow task show <ID>` for full context (done_when, files, refs). If `refs` exists, read corresponding SPEC sections.
- Only do tasks listed in `dow task list` — no more, no less.
- After completing a task, run `dow task done <ID>` then `/devtest`.
- After fixing an issue, run `dow issue close <ID>` immediately — do not leave resolved issues open.
- After all tasks complete, auto-enter `/test`.

### Handling ad-hoc requests during DEV
When receiving a new user message during DEV, assess its relationship to the current task before acting:
- complexity: S (≤2 files, no interface/architecture change) / M (multiple files or local flow) / L (interface/architecture/dependency/task boundary)
- relation: supplement (extends current task) / disruptive (overturns current task premise/approach/acceptance) / independent (unrelated to current task)

Rules:
- S + supplement → update current task notes/done_when, continue DEV
- M + supplement → explain impact scope, split new task if needed
- L + supplement → stop DEV, return to SPEC or TASK
- independent → create new task, do not mix into current task
- disruptive → pause current task, determine whether to discard/rewrite/split/return to SPEC

Examples:
- "Add a log line to debug X" → S + supplement → update notes, continue
- "Also need an export CSV button" → M + independent → create new task
- "Can we use Redis instead of in-memory cache?" → L + disruptive → pause, reassess

### .dev-doc management
- Structural files (task/issue/STATUS/CHANGELOG): ALL operations through dow commands. Never Read/Write directly.
- Document files (PRD.md/SPEC.md/BRAINSTORM.md): Create via dow (`dow prd create` etc), edit directly after creation.
- Schema: use `dow <resource> schema` (e.g. `dow task schema`, `dow spec schema`) to get format definitions.
- Batch create: pipe JSON array to stdin — `echo '[{...},{...}]' | dow task create` (same for issue).
- Update fields: `dow task update <ID> --field value` / `dow issue update <ID> --field value` (only passed fields change).
- Remove: `dow task remove <ID>` / `dow issue remove <ID>` (requires confirmation token; renumbers subsequent IDs).

### Role isolation
- BRAINSTORM/PRD/SPEC: main agent writes artifact directly, then spawns audit subagent for independent review.
- TASK: main agent decomposes (low complexity) or spawns adversarial subagents (high complexity).
- TEST: runs in independent agent with strict isolation. Each agent only receives minimal input for that phase.

{CODEX DEV FLOW Discipline}

</dev-flow>