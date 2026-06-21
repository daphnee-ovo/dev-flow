---
source: other
nums: 1
---

- [x] ISSUE-I001：iterate 吞掉 git add 失败导致返回成功但未提交
  - severity: P1
  - location：dow/src/commands/iterate.rs:760
  - description：`git_commit` 对 `git add -u` 和 `git add <file>` 使用 `.output().ok()`，当 git add 因权限、pathspec 或其他错误失败时不会返回错误；随后如果没有 staged changes，iterate 会返回成功，造成归档、版本、阶段已更新但 release commit 未创建的半成品状态。
  - reproduce：让 `.git/index.lock` 无法创建或传入不可 add 的 `--files`，执行 `dow iterate --confirm`。
  - fix：`git_commit` 改为检查 `git add -u` 和 `git add <file>` 的退出状态，失败时返回 DowError；无 staged changes 时返回明确错误；新增测试覆盖 `--files` 指向不存在文件时阻断 iterate commit。
