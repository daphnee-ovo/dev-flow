---
description: 启动 SPEC 阶段 — 技术方案设计
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# SPEC — 技术规范设计

## 前置检查（模式感知）

1. 读取 STATUS.yaml 中的 mode
2. 按模式决定输入源：
   - **full 模式**：检查 `<DOC_ROOT>/PRD.md` 是否存在，不存在 → 停止，告知用户先执行 /prd
   - **quick/mvp 模式**：PRD.md 不要求存在。输入源降级为 BRAINSTORM.md（如有）或用户描述
3. 生成项目上下文：`dow inbox context`

## 输入组装（模式感知）

| 模式 | 需求输入 | 项目上下文 |
|------|----------|-----------|
| full | PRD.md（必须存在） | 始终传入 |
| quick | BRAINSTORM.md（如有）或用户描述 | 始终传入 |
| mvp | BRAINSTORM.md（如有）或用户描述 | 始终传入 |

## Agent 调度（隔离模板）

**必须启动独立子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "SPEC agent - 技术规范设计"
prompt: `<读取 agents/spec-agent.md 的完整内容>

## 输入文档

### 需求来源
<按模式传入：PRD.md 完整内容 / BRAINSTORM.md 内容 / 用户描述>

### 项目上下文
<执行 dow inbox context 的输出，原样粘贴>

## 输出路径

将 SPEC 写入：<DOC_ROOT>/SPEC.md

## 禁止

- 不要阅读无关的历史文件
- 不要参考 PRD/BRAINSTORM 的讨论过程（你看不到，也不需要）
- 不要拆解任务（那是 TASK 阶段的事）
- 不要开始写代码`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| agents/spec-agent.md 内容 | PRD/BRAINSTORM 阶段的对话讨论过程 |
| PRD.md / BRAINSTORM.md 内容（按模式） | 用户与 agent 的交互历史 |
| 项目上下文（context.sh 输出） | TASK.md / TEST.md |
| DOC_ROOT 路径 | 无关历史记录 |

## 隔离边界说明

隔离的是**讨论过程**，不是**项目现状**。SPEC agent 需要了解项目当前结构（目录、技术栈、已有模块）才能做出合理的架构决策，但不应看到需求讨论中被否决的方案。

## 完成后

1. 确认 SPEC.md 已写入
2. 更新 STATUS.yaml：当前阶段 → SPEC
3. 提示用户：确认 SPEC 后执行 `/task` 推进

## 输出约束

- SPEC 保持轻量，默认包含 Goal、Scope、Requirements Trace、Design、Acceptance、Risks、Test Plan、Self Check。
- quick/fast/mvp 按模式降级，不为了模板完整性补无用章节。
- Change 直接写在 Requirements Trace 的 Notes 里，不单独创建 Change Delta。
- 不要开始写代码，不要拆 task。
