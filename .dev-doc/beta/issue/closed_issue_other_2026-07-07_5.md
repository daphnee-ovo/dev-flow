---
source: other
nums: 1
---

- [x] ISSUE-I006：增量数组语法未更新到 agent 提示和 CLI help
  - severity: P1
  - location：plugin/commands/task.md:0
  - description：新增的 +item/-item 增量语法没有更新到 plugin commands、全局 CLAUDE.md dev-flow 块、CLI help text，agent 不知道该功能
  - reproduce：agent 在 DEV 阶段需要追加 files_modify 时仍用全量替换
  - fix：更新 inject_prompt/dev_flow.md 增加 incremental array syntax 说明、plugin/commands/task.md 和 issue.md 增加增量语法文档、cli.rs help text 增加 +item/-item 提示
  - files_modify: [dow/references/inject_prompt/dev_flow.md, plugin/commands/task.md, plugin/commands/issue.md, dow/src/cli.rs]
  - files_create: []
