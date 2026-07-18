# Sample Project Walkthrough

This fixture shows what dev-flow leaves behind after a small feature cycle after `/iterate`.

Scenario: an agent is asked to add a settings page to a small app. dev-flow keeps the workflow state in `.dev-doc/` instead of relying only on chat history.

The `.dev-doc` files in this example were generated in an isolated temporary project with:

```bash
dow init --name sample-settings-app --mode fast
dow task schema
dow issue schema
dow hooks post-write .dev-doc/main/task/task_2026-06-08_1.md
dow hooks post-write .dev-doc/main/issue/issue_test_2026-06-08_1.md
dow iterate --topic settings-page --type feat --files src tests
dow iterate --topic settings-page --type feat --files src tests --confirm ITR-xxxxxx
```

## Files To Inspect

```text
examples/sample-project/
├── .dev-doc/
│   ├── archive.db
│   └── main/
│       ├── CHANGELOG.md
│       └── STATUS.yaml
├── archive-queries.md
├── src/
│   └── settings.ts
└── tests/
    └── settings.test.ts
```

## Workflow

1. `/init` creates `.dev-doc/STATUS.yaml`.
2. `/task` turns the feature request into a structured task file.
3. Development updates code and marks task checkboxes only after verification.
4. `dow test <TASK-ID>` validates the task; creates an issue if tests fail.
5. `/iterate` archives completed tasks, issues, BRAINSTORM, PRD, SPEC, and CHANGELOG records into `.dev-doc/archive.db`.
6. `dow archive ...` commands query archived records from SQLite.

See [archive-queries.md](archive-queries.md) for the real `dow archive` output from this sample.

The example is static and safe to inspect. It is not intended to be a runnable application.
