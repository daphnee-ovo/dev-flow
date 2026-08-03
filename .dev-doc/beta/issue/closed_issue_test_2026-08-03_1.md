---
source: test
nums: 1
---

- [x] ISSUE-I002：4 stale test assertions in test_dow_branch.rs due to message text drift
  - severity: P1
  - location：dow/tests/test_dow_branch.rs
  - description：Tests assert on old message text (e.g. 'dow task create') that was changed in context.rs/guard.rs to generic wording ('create a task/issue'). 4 tests fail: test_context_blocks_when_all_tasks_done, test_context_codex_hook_injects_context_without_blocking, test_guard_blocks_code_write_when_all_done, test_save_changelog_codex_hook_outputs_stop_json.
  - reproduce：cargo test --manifest-path dow/Cargo.toml --test test_dow_branch
  - fix：Updated assertions to match current message text; renamed save-changelog to session-stop in test.
  - files_modify: [dow/tests/test_dow_branch.rs]
  - files_create: []
