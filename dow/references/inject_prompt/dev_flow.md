<dev-flow>

# **MUST use DEV-FLOW to manage development workflow.**

## Commands
| Phase | Slash Command | CLI Command | Notes |
|---|---|---|---|
| any | `/init` | `dow init` | Initialize project |
| any | `/status` | `dow status` | Report current status |
| any | `/check` | `dow lint` | Check doc sync |
| any | `/mode` | `dow status set --mode` | Select dev mode |
| any | `/issue` | `dow issue create` | Create an issue |
| BRAINSTORM | `/brainstorm` | `dow brainstorm create` (entry) | Interactive exploration → creates BRAINSTORM.md |
| PRD | `/prd` | `dow prd create` (entry) | Interactive → creates PRD.md, then spawns audit |
| SPEC | `/spec` | `dow spec create` (entry) | Interactive → creates SPEC.md, then spawns audit |
| TASK | `/task` | `dow task create` (entry) | Decompose spec into tasks, batch create |
| DEV | `/fix` | `dow issue list` + `dow claim` | Read open issues, claim, fix, close |
| DEV | `/devtest` | `dow test --task <ID>` | Task-level testing loop |
| TEST | `/test` | `dow test` | Full project-level test suite |
| ITERATE | `/iterate` | `dow iterate` | Preview → confirm → archive + commit + tag + bump |

## Discipline

### Before writing code
- **Sequence: confirm intent → create task/issue → claim → code.**
  1. First confirm the user's intent — do NOT assume or start work without explicit approval.
  2. Create a task (`dow task create`) or issue (`dow issue create`) that matches the request.
  3. Run `dow claim <TASK-ID or ISSUE-ID>` to declare your work target.
  4. Only then start writing code.
- Claims expire after 5 minutes — re-claim if still working.
- Only claim tasks/issues directly related to the current user request. Do NOT claim unrelated items — if the user's request doesn't map to an existing task/issue, create a new one instead of claiming an irrelevant one.
- After completing the task, run `dow claim --revoke` to release.

### DEV phase rules
- **IMPORTANT: Never modify code without an open task or issue.** If none exists, create one first (`dow task create` or `dow issue create`) before writing any code.
- When hook output contains `[BLOCKED]`, stop all code modifications. Flow management commands (`dow task create`, `dow issue create`, `dow status`, `/iterate`) remain available.
- Before starting a task, use `dow task show <ID>` for full context (done_when, files, refs). If `refs` exists, read corresponding SPEC sections.
- Do not work on items outside `dow task list`. New requests must first become a task/issue before work begins.
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
- Commands with `--confirm` tokens (remove, reopen, iterate) are destructive or hard to reverse. NEVER generate or execute them without explicit user approval — always show the action and token to the user first and wait for confirmation. The preview and the confirmed execution MUST happen in separate turns — presenting the preview and executing in the same turn makes user confirmation meaningless.

### Role isolation
- BRAINSTORM/PRD/SPEC: main agent writes artifact directly, then spawns audit subagent for independent review.
- TASK: main agent decomposes (low complexity) or spawns adversarial subagents (high complexity).
- TEST: runs in independent agent with strict isolation. Each agent only receives minimal input for that phase.

{CODEX DEV FLOW Discipline}

</dev-flow>