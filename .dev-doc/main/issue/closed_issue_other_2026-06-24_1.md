---
source: other
nums: 1
---

- [x] ISSUE-I010：dow revoke 子命令重命名为 dow rollback
  - severity: P2
  - location：dow/src/cli.rs:82
  - description：revoke 语义不够明确，rollback 更直观地表达"版本回退"的意图。需要重命名 CLI 子命令、Rust 源码文件/结构体、文档引用和测试文件。
  - fix：已完成重命名：CLI 子命令 Revoke→Rollback、源文件 revoke.rs→rollback.rs、结构体 RevokeArgs/RevokeOutput→RollbackArgs/RollbackOutput、所有文档引用（CLAUDE.md/README/docs/*）、测试文件

