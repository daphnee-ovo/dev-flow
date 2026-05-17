---
description: 启动完整 TEST 阶段 — 项目级全量验证
allowed-tools: Agent, Bash, Read, Write, Edit
---

# TEST — 项目测试（全量验证）

## 前置检查（阻断）

1. 读取 `task/` 目录下所有活跃 task 文件，统计未完成任务数量
2. 如果有未完成任务 → **停止，告知用户先完成所有任务**，不继续
3. 检查 `dev-doc/issue/` 中是否有未关闭的 issue → 提醒先修复

## 阶段切换

**在启动 agent 之前**，立即更新 STATUS.yaml 为 TEST。

## Agent 调度（隔离模板）

**启动全新独立 TEST agent（完整版），绝对不复用开发上下文。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "项目 TEST - 全量验证"
prompt: `<读取 agents/test-agent.md 的完整内容>

## 输入文档

### SPEC.md（验证标准）
<SPEC.md 完整内容，原样粘贴>

### Task 文件（验证范围）
<task/ 目录下所有未完成 task 文件的内容，原样粘贴>

## 输出路径

- 测试代码：tests/（按模块分目录，test_<功能>.py）
- 测试报告：<DOC_ROOT>/TEST.md
- Issue 文件：<DOC_ROOT>/issue/issue_test_<YYYY-MM-DD>_<seq>.md（每个问题单独一个文件）

## 禁止

- 不要查看 git log 或 commit 历史
- 不要参考开发过程中的任何对话
- 不要因为 SPEC 没要求的功能缺失而报 issue
- 不要因为 TASK 标记为"非目标"的内容报 issue
- 不要信任"开发者说测过了"——自己验证`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| agents/test-agent.md 内容 | PRD.md |
| SPEC.md 完整内容（原文） | 开发阶段的任何对话历史 |
| task/ 目录下所有 task 文件内容 | git log / commit messages |
| DOC_ROOT 路径 | 例行 TEST 的结果历史 |
| 当前日期（用于 issue 文件名） | |

## 为什么严格隔离

TEST agent 必须**完全独立于开发 agent**。开发者会无意识地避开自己代码的薄弱点。只有全新视角、只看文档不看过程的独立测试，才能发现真正的盲区。

这不是形式主义——这是流程中最关键的质量保证。

## 结果处理

- **全部通过** → 执行 /done
- **发现问题** → issue 文件已写入，STATUS 切回 DEV，修复后重新 /test
