# Sample Project Walkthrough

This fixture shows what dev-flow leaves behind after a small feature cycle.

Scenario: an agent is asked to add a settings page to a small app. dev-flow keeps the workflow state in `.dev-doc/` instead of relying only on chat history.

## Files To Inspect

```text
examples/sample-project/
├── .dev-doc/
│   ├── STATUS.yaml
│   ├── task/
│   │   └── done_task_2026-06-08_1.md
│   ├── issue/
│   │   └── closed_issue_devtest_2026-06-08_1.md
│   └── archive/
│       └── v1-settings-page/
│           ├── SUMMARY.md
│           └── TEST.md
├── src/
│   └── settings.ts
└── tests/
    └── settings.test.ts
```

## Workflow

1. `/init` creates `.dev-doc/STATUS.yaml`.
2. `/task` turns the feature request into a structured task file.
3. Development updates code and marks task checkboxes only after verification.
4. `/devtest` records a concrete issue if validation fails.
5. `/iterate` archives the completed cycle under `.dev-doc/archive/`.

The example is static and safe to inspect. It is not a runnable application.

