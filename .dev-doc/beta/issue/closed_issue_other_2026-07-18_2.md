---
source: other
nums: 1
---

- [x] ISSUE-I002：feat: add VERSION (main) sync step to preIterate.ci
  - severity: P1
  - location：.dev-doc/preIterate.ci:1
  - description：iterate 前需要将 (main) 设为当前 (beta) 版本，确保 PR 入 main 时版本正确
  - reproduce：当前 preIterate.ci 缺少 VERSION (main) 同步步骤
  - fix：在 preIterate.ci 的 test 之后、sync-version 之前加入 sed 命令，将 VERSION 中 (main) 设为当前 (beta) 值
  - files_modify: [.dev-doc/preIterate.ci]
  - files_create: []
