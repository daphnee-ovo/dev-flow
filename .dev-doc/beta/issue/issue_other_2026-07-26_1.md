---
source: other
nums: 1
---

- [ ] ISSUE-I002：Rollback does not renumber newly created tasks causing ID collision
  - severity: P2
  - location：dow/src/commands/rollback.rs
  - description：After iterate archives tasks (T001-T004), new task creation resets to T001. Subsequent rollback restores old done_tasks, causing duplicate T001 IDs (one done, one pending).
  - reproduce：1. iterate (archives T001-T004) 2. create new task (gets T001) 3. rollback → two T001s exist
  - fix：
  - files_modify: [dow/src/commands/rollback.rs]
