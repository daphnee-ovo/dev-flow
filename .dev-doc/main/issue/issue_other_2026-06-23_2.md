---
source: other
nums: 4
---

- [x] ISSUE-I003：dow fix 不处理 issue closed_ 重命名
  - severity: P1
  - location：dow/src/commands/fix.rs
  - description：validate 检测到"所有 issue 已勾选但文件未重命名为 closed_ 前缀"并标记 fixable: true，post-write hook 已有重命名实现，但 dow fix 没有对应逻辑。当 hook 未触发时（手动编辑、git checkout）状态不一致无法自动修复。对应 GitHub #12
  - fix：新增 fix_issue_rename 函数，扫描全部勾选的 issue 文件自动加 closed_ 前缀
- [x] ISSUE-I004：dow fix 不处理 task done_ 重命名
  - severity: P1
  - location：dow/src/commands/fix.rs
  - description：同 closed_ 场景，task 文件所有 checkbox 勾选后应自动加 done_ 前缀，post-write hook 有实现但 dow fix 缺失
  - fix：新增 fix_task_rename 函数，逻辑与 fix_issue_rename 对称
- [x] ISSUE-I005：dow fix 不处理历史 issue 全局序号冲突
  - severity: P1
  - location：dow/src/commands/fix.rs
  - description：旧版本每个 issue 文件独立从 ISSUE-I001 编号，升级后全局序号校验报冲突（14个文件39条issue），阻断 iterate。dow fix 应能按文件日期排序自动重编号。对应 GitHub #13
  - fix：新增 fix_issue_renumber 函数，按文件日期+序号排序后重新分配全局连续 ISSUE-I 编号
- [x] ISSUE-I006：dow iterate --files 文档未说明不需要传入 .dev-doc 下的文件
  - severity: P1
  - location：plugin/commands/iterate.md
  - description：用户误将 closed_issue 等会被归档删除的文件传入 --files，导致归档后 git add 该路径失败。根因是文档未说明 .dev-doc 下的文件由 iterate 自动管理（git add -u 已覆盖），--files 只需传入额外的源码文件。对应 GitHub #11
  - fix：更新 iterate.md 文档说明 --files 无需传入 .dev-doc 文件；同时在 git_commit 中跳过不存在的路径作为防御

