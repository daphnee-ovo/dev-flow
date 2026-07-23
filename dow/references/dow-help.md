dev-flow unified CLI dispatcher.

All hooks and scripted operations execute through dow. Default JSON output, -H switches to human-friendly format.

Process:
  [Brainstorm] → PRD → SPEC → TASK → DEV → TEST → ITERATE → Next Round

Modes:
  full    PRD → SPEC → TASK → DEV → TEST → ITERATE
  quick   SPEC → TASK → DEV → TEST → ITERATE
  fast    TASK → DEV → TEST → ITERATE
  mvp     SPEC → TASK → DEV → ITERATE
  audit   Auto-triggered for urgent issues in non-DEV phases

DEV phase rules:
  - When hook output contains [BLOCKED], stop all dev operations.
  - Before starting a task, run `dow claim <TASK-ID>` (expires 5 min; use `--timeout <secs>` for longer tasks, max 600).
  - Use `dow task show <ID>` for context (done_when, files, refs) before coding.
  - Only do tasks listed in `dow task list` — no more, no less.
  - `dow task done <ID>` auto-runs tests and auto-revokes claim on success. After all tasks, run `dow test`.

.dev-doc management:
  Structural files (task/issue/STATUS/CHANGELOG): ALL operations through dow commands.
  Document files (PRD.md/SPEC.md/BRAINSTORM.md): Create via dow, edit directly after creation.
  Schema: `dow <resource> schema` for format definitions.

Role isolation:
  PRD/SPEC/TASK/TEST phases run in independent agents with minimal input.
  DEV phase runs in main agent directly.

preIterate CI (.dev-doc/preIterate.ci):
  Runs before git commit during iterate. Any step failure blocks the entire iterate.
  Supported steps:
    sync-version: <path>   Sync manifest version (Cargo.toml, package.json, pyproject.toml)
    run: <command>          Execute command; non-zero exit blocks iterate

Dashboard:
  `dow dashboard [--port PORT]` launches a local web visualization panel.
  Shows project status, task dependency graph (interactive), docs, kanban board.
  SSE real-time updates. Auto-exits when all browser tabs close.
  Right-click drag to pan, scroll wheel to zoom in the graph.

Directory structure:
  .dev-doc/
  ├── STATUS.yaml
  ├── CHANGELOG.md
  ├── BRAINSTORM.md, PRD.md, SPEC.md
  ├── task/       (task_*.md, done_task_*.md)
  ├── issue/      (issue_*.md, closed_issue_*.md)
  └── archive.db  (SQLite, queried via `dow archive`)
