---
source: other
nums: 1
---

- [x] ISSUE-I001：iterate token 未绑定命令参数
  - severity: P1
  - location：dow/src/commands/iterate.rs:493-513
  - description：token 仅基于 cwd + 时间分钟生成，不含 topic/type/bump/files，预览后修改参数仍可用同一 token confirm
  - reproduce：连续两次不同参数预览，token 相同
  - fix：将 args.topic/type/bump/files 混入 token 哈希，不同参数组合产生不同 token

