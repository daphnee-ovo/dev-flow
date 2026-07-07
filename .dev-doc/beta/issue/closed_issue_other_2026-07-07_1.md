---
source: other
nums: 1
---

- [x] ISSUE-I002：dow issue show/list 不显示 files_modify/files_create 字段
  - severity: P1
  - location：dow/src/commands/issue.rs:637
  - description：IssueShowOutput 结构体缺少 files_modify 和 files_create 字段，find_issue_by_id 已解析但 show() 未传递到输出；list 的 IssueItem 同理
  - reproduce：dow issue update <id> --files-modify foo.rs 后执行 dow issue show <id>，输出中无 files 信息
  - fix：在 IssueShowOutput 添加 files_modify/files_create 字段，show() 函数传递 parsed 的值到输出，human 模式条件输出
  - files_modify: [dow/src/commands/issue.rs]
  - files_create: []
