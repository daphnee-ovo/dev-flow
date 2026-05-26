---
description: 迭代交付 — 交付检查 + 归档 + commit & tag + bump 版本
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# ITERATE — 迭代交付

## 总则

`/iterate` 是 dev-flow 的迭代收尾命令。执行后完成当前版本的交付并开启下一个迭代。
包含原 `/done`（交付检查）和原 `/iterate`（归档 + 重置）的全部职责。

## 前置检查（阻断）

脚本自动执行，任一不通过则停止：
1. task 文件中所有任务必须全部勾选 `[x]`
2. 无未关闭的 P0 issue
3. VERSION 文件存在且格式合法

## 参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| topic | 归档主题（用于目录命名） | 必填 |
| bump_type | 版本递增类型：major/minor/patch | minor |

## 执行流程

### 阶段 1：交付检查
- 检查 task 全完成 + 无 P0 issue
- 不通过 → 报错退出

### 阶段 2：读取版本
- 从 `VERSION` 文件读取当前版本号
- 校验格式合法性

### 阶段 3：归档
- 归档目录命名：`dev-doc/archive/v<当前版本>-<topic>/`
- 移动：done_task_*、已全部完成的 task_*、closed_issue_*、CHANGELOG.md
- 复制：PRD.md、SPEC.md、TEST.md
- `/iterate` 前会阻断未完成 task；因此能执行归档时，当前 task_* 已全部完成并会进入 archive
- 未关闭的非 P0 issue 保留在当前 issue 目录，继续跟进
- BRAINSTORM.md 默认不归档（持久参考）

### 阶段 4：用户确认
- 向用户展示变更摘要（归档内容、版本号、将打的 tag）
- 用户确认后继续，否则停止

### 阶段 5：commit & tag
- `git add` 所有变更（代码 + 文档 + 归档）
- `git commit -m "Release v<版本>: <topic>"`
- `git tag -a "v<版本>" -m "Release v<版本>"`

### 阶段 6：bump 版本 + 开启新迭代
- 按 bump_type 递增版本号写入 VERSION
- 重置 STATUS.yaml phase（按 mode 确定初始阶段）
- `git commit -m "Start v<新版本> iteration"`

## Bump 类型决策

1. 默认 minor（每次迭代 = 新功能周期）
2. 用户指定 `--major` → major
3. Agent 检测到架构重构/破坏性变更 → 推荐 major，询问用户确认

## 执行方式

```bash
# 由 agent 调用（确认后设置 DEVFLOW_NO_CONFIRM=1）
DEVFLOW_NO_CONFIRM=1 bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/iterate.sh" "<topic>" "<bump_type>"
```

agent 在调用前：
1. 询问用户本轮迭代主题
2. 判断 bump 类型（默认 minor，检测到大变更推荐 major）
3. 先不带 DEVFLOW_NO_CONFIRM 运行脚本，展示阶段 4 的摘要输出
4. 获取用户确认后设置 `DEVFLOW_NO_CONFIRM=1` 再次执行完整流程

## 注意

- 归档是复制（主文档）+ 移动（task/done_task/closed_issue/CHANGELOG），当前目录被重置
- 如果 archive 目录已存在同名，说明重复操作，脚本会停止并报错

## 完成后输出

```
[dev-flow] 迭代完成
━━━━━━━━━━━━━━━━━━━━━━
交付版本：v2.2.0 (tagged)
新版本：v2.3.0
阶段重置：SPEC
模式：mvp
```
