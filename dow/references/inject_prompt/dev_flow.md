<dev-flow>

# **MUST use DEV-FLOW to manage development workflow.**

## Slash Commands
| Command | Phase | Purpose |
|---------|-------|---------|
| `/init` | any | Initialize project |
| `/status` | any | Report status |
| `/mode` | any | Set dev mode |
| `/issue` | any | Create issue |
| `/brainstorm` | BRAINSTORM | Collaborative exploration |
| `/prd` | PRD | Write requirements |
| `/spec` | SPEC | Technical design |
| `/task` | TASK | Decompose into tasks |
| `/fix` | DEV | User-triggered workflow to read, claim, fix, verify, and close open issues |
| `/test` | TEST | Full project test; Task tests use dow test TASK-ID |
| `/iterate` | ITERATE | Archive + commit + tag + bump |

## Discipline

### Before writing code
- **Sequence: confirm intent → create task/issue → claim → code.**
  1. First confirm the user's intent — do NOT assume or start work without explicit approval.
  2. Create a task (`dow task create`) or issue (`dow issue create`) that matches the request.
  3. Run `dow claim <TASK-ID or ISSUE-ID>` to declare your work target.
  4. Only then start writing code.
- Claims expire after 5 minutes (default) — re-claim if still working. Use `--timeout <secs>` (max 600) for longer tasks.
- `/fix` is user-triggered. Do not start it automatically from hooks, tests, checks, task completion, or issue creation.
- Only claim tasks/issues directly related to the current user request. Do NOT claim unrelated items — if the user's request doesn't map to an existing task/issue, create a new one instead of claiming an irrelevant one.
### DEV phase rules
- **IMPORTANT: Never modify code without an open task or issue.** If none exists, create one first (`dow task create` or `dow issue create`) before writing any code.
- When hook output contains `[BLOCKED]`, stop all code modifications. Flow management commands (`dow task create`, `dow issue create`, `dow status`, `/iterate`) remain available.
- Before starting a task, use `dow task show <ID>` for full context (done_when, files, refs). If `refs` exists, read corresponding SPEC sections.
- Do not work on items outside `dow task list`. New requests must first become a task/issue before work begins.
- **Completion sequence (MUST execute immediately, no delay):**
  - Task done: `dow task done <ID>` (auto-runs tests, auto-revokes claim on success)
  - Issue fixed: `dow issue close <ID>` (auto-revokes claim on success)
  - Do NOT defer these commands. Execute them as soon as the work is verified complete — before moving to the next task, before responding to the user, before any other action.
- After all tasks complete, ask whether the user wants to enter `/test`. Never enter TEST or launch the TEST agent without an explicit user request.

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
- **When unsure about any dow command syntax, run `dow <command> --help` first.**
- Commands with `--confirm` tokens (remove, reopen, iterate) are destructive or hard to reverse. NEVER generate or execute them without explicit user approval — always show the action and token to the user first and wait for confirmation. The preview and the confirmed execution MUST happen in separate turns.

#### dow task/issue command reference

| Operation | Command | Notes |
|-----------|---------|-------|
| create | `echo '<JSON>' \| dow task create` | pipe JSON object or array to stdin; all fields required |
| list | `dow task list [--all]` | default shows pending only |
| show | `dow task show <ID>` | full detail with files, deps, done_when |
| update | `dow task update <ID> --field value` | only passed fields change; array fields support incremental syntax |
| done/close | `dow task done <ID>` / `dow issue close <ID>` | marks complete |
| remove | `dow task remove <ID>` | preview → confirm token (destructive) |
| reopen | `dow task reopen <ID>` | preview → confirm token (destructive) |
| schema | `dow task schema` | outputs field definitions |

Replace `task` with `issue` for issue operations (same pattern).

#### update — incremental array syntax

Nested `files` arrays, `depends_on`, and `done_when` support incremental operations:
- `+item` → append (deduplicated)
- `-item` → remove

```bash
dow task update T001 --file '{"modify":["+new.rs","-old.rs"]}'   # append new.rs, remove old.rs
dow task update T001 --done-when "+新验收标准"            # append criterion
```

If NO item has `+`/`-` prefix → full replacement (backward compatible). Mixing prefixed and unprefixed in one list is not supported — either all incremental or all full replace.

#### task create — nested file scope

```json
{"title":"string", "type":"feat|fix|refactor|docs|perf|test|style", "priority":"P0|P1|P2", "refs":"string", "files":{"create":[], "modify":["src/a.rs"], "test":[]}, "depends_on":[], "parallel":false, "complexity":"S|M|L", "done_when":["criterion"]}
```
`files.create` and `files.modify` are individually optional, but at least one
must contain a non-empty path. CLI input uses `--file '{"modify":["src/a.rs"]}'`.

#### issue create — nested file scope

```json
{"title":"string", "severity":"P0|P1|P2", "location":"file:line", "desc":"string", "reproduce":"string", "source":"test|audit|other", "files":{"create":[], "modify":["src/a.rs"]}}
```
Issue `files` has no `test` category. Legacy flat file fields and `--files-*`
flags are not accepted. CLI and stdin input cannot be combined.

### Feedback
- If you encounter any issue while using dev-flow (unexpected behavior, command failure, documentation mismatch, etc.), and the user permits, create a GitHub issue directly in the dev-flow repository (`daphnee-ovo/dev-flow`) to report it. Use `gh issue create --repo daphnee-ovo/dev-flow --title "<title>" --body "<description>"`.

### Role isolation
- BRAINSTORM/PRD/SPEC: main agent writes artifact directly, then spawns audit subagent for independent review.
- TASK: main agent decomposes (low complexity) or spawns adversarial subagents (high complexity).
- TEST: runs in an independent agent with strict isolation, but only after the user explicitly invokes `/test` or asks to enter the TEST phase. 

{CODEX DEV FLOW Discipline}

</dev-flow>
