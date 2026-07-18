---
source: other
nums: 1
---

- [x] ISSUE-I001：fix: remove VERSION consistency check from PR gate, keep branch restriction only
  - severity: P1
  - location：.github/workflows/check-version.yml:19
  - description：VERSION 一致性检查逻辑不符合实际流程，需移除，仅保留来源分支限制
  - reproduce：beta 开发中 (main) != (beta) 是正常状态，但 CI 会误报
  - fix：移除 VERSION 一致性检查，仅保留来源分支限制（只允许 beta PR 入 main）
  - files_modify: [.github/workflows/check-version.yml]
  - files_create: []
