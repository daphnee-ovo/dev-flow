---
description: 启动 SPEC 阶段 — 技术方案设计
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# SPEC — 技术规范设计

## 前置检查（阻断）

1. 检查 `<DOC_ROOT>/PRD.md` 是否存在
2. 如果不存在 → **停止，告知用户先执行 /prd**，不继续

## Agent 调度（隔离模板）

**必须启动独立子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "SPEC agent - 技术规范设计"
prompt: `<读取 agents/spec-agent.md 的完整内容>

## 输入文档

<仅传入 PRD.md 的完整内容，原样粘贴，不做任何摘要或改写>

## 输出路径

将 SPEC 写入：<DOC_ROOT>/SPEC.md

## 禁止

- 不要阅读无关的历史文件
- 不要参考 PRD 的讨论过程（你看不到，也不需要）
- 不要拆解任务（那是 TASK 阶段的事）
- 不要开始写代码`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| agents/spec-agent.md 内容 | PRD 阶段的对话讨论过程 |
| PRD.md 完整内容（原文） | 用户与 PRD agent 的交互历史 |
| DOC_ROOT 路径 | 任何代码文件内容 |
| | TASK.md / TEST.md |
| | 无关历史记录 |

## 为什么严格隔离

SPEC agent 只看"最终需求是什么"，不看"需求是怎么讨论出来的"。PRD 讨论中被否决的方案、犹豫过的功能，如果 SPEC agent 看到了，会干扰它做出纯粹的架构判断。

## 完成后

1. 确认 SPEC.md 已写入
2. 更新 STATUS.yaml：当前阶段 → SPEC
3. 提示用户：确认 SPEC 后执行 `/task` 推进
