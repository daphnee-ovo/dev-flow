---
description: 启动 TASK 阶段 — 任务拆解
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# TASK — 任务拆解

## 前置检查（模式感知）

1. 读取 STATUS.yaml 中的开发模式
2. 按模式决定输入源：
   - **full/quick 模式**：检查 `<DOC_ROOT>/SPEC.md` 是否存在，不存在则停止，告知用户先执行 /spec
   - **fast 模式**：不要求 SPEC.md，使用用户描述 + 项目上下文作为输入
3. 生成项目上下文：`bash "${CLAUDE_PLUGIN_ROOT}/scripts/lib/context.sh"`

## 输入组装（模式感知）

| 模式 | 方案输入 | 项目上下文 |
|------|----------|-----------|
| full/quick | SPEC.md（必须存在） | 始终传入 |
| fast | 用户描述（无需 SPEC.md） | 始终传入 |

## Agent 调度（隔离模板）

**必须启动独立子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "TASK agent - 任务拆解"
prompt: `<读取 agents/task-agent.md 的完整内容>

## 输入文档

### 技术方案
<按模式传入：SPEC.md 完整内容 / 用户描述>

### 项目上下文
<执行 scripts/lib/context.sh 的输出，原样粘贴>

## 输出路径

将任务清单写入：<DOC_ROOT>/task/task_<YYYY-MM-DD>_<seq>.md

如果 task/ 目录不存在，先创建它。
seq 为当天该目录下的下一个序号。

## 输出格式

文件使用以下格式：

---
title: TASK - <批次主题>
nums: <任务总数>
---

- [ ] T1：<标题>
  - level: P0
  - model: cheap | standard | capable
  - details：<描述>
  - depends on：无
  - Done when：<完成标准>

## 禁止

- 不要阅读 PRD.md（你不需要知道"为什么做"，只需要知道"怎么做"）
- 不要参考 SPEC 的讨论过程
- 不要开始写代码
- 不要设计新的架构方案`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| agents/task-agent.md 内容 | PRD.md |
| SPEC.md 完整内容（按模式） | PRD/SPEC 阶段的对话历史 |
| 项目上下文（context.sh 输出） | 无关历史记录 |
| DOC_ROOT 路径 | |

## 隔离边界说明

隔离的是**讨论过程**，不是**项目现状**。TASK agent 需要了解项目目录结构和已有模块才能合理评估任务粒度，但不应看到 SPEC 是如何讨论出来的。

## 完成后

1. 确认 task 文件已写入 `<DOC_ROOT>/task/`
2. 更新 STATUS.yaml：当前阶段 → TASK
3. 提示用户：确认任务清单后，STATUS 将切换为 DEV，开始开发
