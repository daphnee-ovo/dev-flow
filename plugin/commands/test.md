---
description: Enter TEST phase and run full-project tests only when the user explicitly requests TEST phase
allowed-tools: Bash, Read
---

# TEST — Test execution

`/test` is the explicit TEST-phase workflow entry. Do not invoke it from Task
completion or a hook reminder; ask the user to enter TEST instead. The `dow test`
CLI is the only test executor and the only component that creates test-failure
ISSUE files.

## Commands

Full project test:

```bash
dow test
```

Task-scoped test:

```bash
dow test TASK-ID
```

`dow test TASK-ID` reads the matching Task from both active `task_*` and
completed `done_task_*` files, then runs its `files.test`. An empty test list is
`PASS`. Paths are relative to `project_root`.

## Configuration

Create `.dev-doc/test.ci` when project defaults do not fit:

```text
devtest:
  run: <Task test command>
test:
  run: <full project test command>
```

The command runs in `project_root` with inherited environment variables.
Available placeholders are ``project_root``, ``task_id``,
``task_file``, and ``test_files``. Unknown placeholders and missing tools
are `PRECONDITION_FAILED`.

Without custom commands, the CLI uses built-in adapters for Rust, Go, Python
pytest, JavaScript/TypeScript package test scripts and runners, and compatible
Shell tests. Unsupported files or runners are precondition failures; they are
not silently executed as Shell.

## Outcomes

- `PASS` exits 0.
- `TEST_FAILED` exits 1, returns the original test output, and creates a P1
  ISSUE with `source: test`.
- `PRECONDITION_FAILED` exits 2, returns the prerequisite error, and does not
  create an ISSUE.

The ISSUE title is `Test fail:<summary>` for full tests and
`Test TASK-ID fail:<summary>` for Task tests. Generated ISSUE Markdown may
contain `files_modify` and `files_create`; public issue create/update input
uses the nested `files` object.

There is no `--file` or `--task` test selector. Use the language's own command
for an ad hoc single-file check, or configure `test.ci`.
