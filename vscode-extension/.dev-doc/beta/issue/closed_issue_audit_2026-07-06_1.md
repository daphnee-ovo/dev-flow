---
source: audit
nums: 1
---

- [x] ISSUE-I001：Guard file scope 检查未读取 issue files
  - severity: P0
  - location：dow/src/hooks/guard.rs:552
  - description：check_claim_file_scope 只查 task files，claim issue 时 allowed_files 为空直接跳过
  - reproduce：claim issue 后写声明外文件无 warning
  - fix：check_claim_file_scope 现在区分 T/I 前缀，issue claim 时通过 get_issue_files 读取 issue 的 files_modify/files_create
  - files_modify: [dow/src/hooks/guard.rs]
  - files_create: []
