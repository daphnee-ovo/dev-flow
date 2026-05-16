---
description: 启动 TASK 阶段 — 任务拆解
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# TASK — 任务拆解

## 前置检查（阻断）

1. 读取 STATUS.md 中的开发模式
2. 如果模式为 `full` 或 `quick`：检查 `<DOC_ROOT>/SPEC.md` 是否存在，不存在则停止，告知用户先执行 /spec
3. 如果模式为 `fast`：不要求 SPEC.md，直接进入任务拆解（基于用户描述或已有代码）

## Agent 调度（隔离模板）

**必须启动独立子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "TASK agent - 任务拆解"
prompt: `<读取 agents/task-agent.md 的完整内容>

## 输入文档

<仅传入 SPEC.md 的完整内容，原样粘贴，不做任何摘要或改写>

## 输出路径

将任务清单写入：<DOC_ROOT>/TASK.md

## 禁止

- 不要阅读 PRD.md（你不需要知道"为什么做"，只需要知道"怎么做"）
- 不要阅读 dev-doc/session/ 下的任何文件
- 不要参考 SPEC 的讨论过程
- 不要开始写代码
- 不要设计新的架构方案`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| agents/task-agent.md 内容 | PRD.md |
| SPEC.md 完整内容（原文） | PRD/SPEC 阶段的对话历史 |
| DOC_ROOT 路径 | 任何代码文件内容 |
| | session/memory/ 中的记录 |

## 为什么严格隔离

TASK agent 只看"技术方案是什么"，基于此独立判断如何拆分。如果看到了 PRD 或之前的讨论，会不自觉地按讨论顺序而非开发顺序来组织任务。

## 完成后

1. 确认 TASK.md 已写入
2. 更新 STATUS.md：当前阶段 → TASK
3. 提示用户：确认任务清单后，STATUS 将切换为 DEV，开始开发
