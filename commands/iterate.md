---
description: 将已交付项目重新激活，进入下一轮迭代
allowed-tools: Bash, Read, AskUserQuestion
---

# ITERATE — 启动新迭代

## 前置检查

1. 确认 `dev-doc/` 存在
2. 如果 STATUS 不是 DONE → **自动触发 /done 检查**
3. /done 通过后继续 iterate 流程
4. /done 未通过 → 停止，告知用户需先解决阻断项

## 执行方式

1. 询问用户本轮迭代主题（用于 archive 命名）
2. 运行脚本：

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/iterate.sh" "<topic>"
```

脚本自动完成归档（done_task_*、closed_issue_*、CHANGELOG.md、主文档副本）和 STATUS.yaml 重置。

## 遗留内容

- 未完成的 task 文件保留在 `dev-doc/task/`，**不归档**
- 未关闭的 issue 保留在 `dev-doc/issue/`，**不归档**
- BRAINSTORM.md 默认不归档（持久参考）

## 注意

- 归档是复制（主文档）+ 移动（done_task/closed_issue/CHANGELOG），当前目录被重置
- 如果 archive 目录已存在同名，说明重复操作，脚本会停止并报错
