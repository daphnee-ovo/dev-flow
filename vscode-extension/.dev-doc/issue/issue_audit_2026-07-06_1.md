---
source: audit
nums: 1
---

- [ ] ISSUE-I001：Guard file scope 检查未读取 issue files
  - severity: P0
  - location：dow/src/hooks/guard.rs:552
  - description：check_claim_file_scope 只查 task files，claim issue 时 allowed_files 为空直接跳过检查
  - reproduce：dow claim ISSUE-ID 后写入 issue 声明外的文件，不会收到 warning
  - fix：
