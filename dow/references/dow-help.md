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
  - Before starting a task, run `dow claim <TASK-ID>` (expires 5 min).
  - Use `dow task show <ID>` for context (done_when, files, refs) before coding.
  - Only do tasks listed in `dow task list` — no more, no less.
  - After task completion, run `dow task done <ID>` then devtest. After all tasks, run test.
  - After completing, run `dow claim --revoke` to release.

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

Directory structure:
  .dev-doc/
  ├── STATUS.yaml
  ├── CHANGELOG.md
  ├── BRAINSTORM.md, PRD.md, SPEC.md
  ├── task/       (task_*.md, done_task_*.md)
  ├── issue/      (issue_*.md, closed_issue_*.md)
  └── archive.db  (SQLite, queried via `dow archive`)
