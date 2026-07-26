---
source: audit
nums: 1
---

- [x] ISSUE-I001：audit mode iterate archives incomplete task files
  - severity: P1
  - location：dow/src/commands/iterate.rs:198
  - description：In audit mode, iterate skips task completion check but still archives and deletes all task_* files alongside done_task_* files. Incomplete tasks should be preserved for the next iteration rather than archived and deleted.
  - reproduce：1. Enter audit mode
    2. Have a task_* file with unchecked items
    3. Run dow iterate with valid params and confirm
    4. Observe: the incomplete task_* file is archived to SQLite and deleted from disk
  - fix：In audit mode, task archiving now skips task_* files (incomplete) and only archives done_task_* files. list_archive_files preview also respects audit mode.
  - files_modify: [dow/src/commands/iterate.rs]
  - files_create: []
