---
source: other
nums: 1
---

- [x] ISSUE-I005：task/issue update 数组字段支持增量语法（+item/-item）
  - severity: P1
  - location：dow/src/commands/task.rs:0
  - description：所有数组字段（files_modify/files_create/files_test/depends_on/done_when）支持 +item 追加、-item 移除的增量操作，无前缀仍为全量替换
  - reproduce：dow task update T001 --files-modify +new.rs 期望追加而非替换
  - fix：在 task.rs 添加 apply_incremental() 函数：判断列表中是否含 +/- 前缀项，有则增量操作，无则全量替换（向后兼容）。task update 和 issue update 的 5+2 个数组字段均通过该函数合并
  - files_modify: [dow/src/commands/task.rs, dow/src/commands/issue.rs]
  - files_create: []
