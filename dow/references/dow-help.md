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
  - Read task file for context (done_when, files, refs) before coding.
  - Only do tasks listed in task/ — no more, no less.
  - After task completion, run devtest. After all tasks, run test.
  - After completing, run `dow claim --revoke` to release.

Doc format:
  When writing .dev-doc files, use `dow doc <type> --json` to get format definition.

Role isolation:
  PRD/SPEC/TASK/TEST phases run in independent agents with minimal input.
  DEV phase runs in main agent directly.

Directory structure:
  .dev-doc/
  ├── STATUS.yaml
  ├── CHANGELOG.md
  ├── BRAINSTORM.md, PRD.md, SPEC.md, TEST.md
  ├── task/       (task_*.md, done_task_*.md)
  ├── issue/      (issue_*.md, closed_issue_*.md)
  └── archive.db  (SQLite, queried via `dow archive`)
