---
description: 启动 PRD 阶段 — 将探索成果格式化为正式需求文档
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# PRD — 产品需求定义

## 执行步骤

1. 检测项目模式，确定 `DOC_ROOT`（见下方脚本）
2. 如果 `dev-doc/` 不存在，创建目录结构
3. 检查是否存在 `BRAINSTORM.md`（决定工作模式）
4. 读取本插件的 `agents/prd-agent.md`
5. **启动独立 Agent（严格按模板）**
6. Agent 完成后，更新 `STATUS.md`

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.md" -path "*/*/STATUS.md" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

## 新项目初始化

```bash
mkdir -p dev-doc/{issue,session/{task,memory}}
```

## 两种工作模式

### 模式 A：有 BRAINSTORM.md（来自 /brainstorm）
- PRD agent 读取 BRAINSTORM.md，提取结构化需求
- 识别缺失信息，逐个向用户确认
- 输出正式 PRD.md

### 模式 B：无 BRAINSTORM.md（直接进入 /prd）
- PRD agent 直接与用户对话式探索需求
- 等同于原来的深度追问模式
- 输出正式 PRD.md

## Agent 调度（强制模板）

**必须使用以下格式启动 Agent，不允许传入额外上下文：**

```
Agent({
  description: "PRD agent - 产品需求定义",
  prompt: `<读取 agents/prd-agent.md 的完整内容>

## 项目信息

<仅传入以下内容>
- 用户本次描述的项目想法（原文）
- 项目名称（如果用户提到了）

## 已有探索成果

<如果 BRAINSTORM.md 存在，粘贴完整内容>
<如果不存在，写"无，需要从零开始探索需求">

## 输出路径

将 PRD 写入：<DOC_ROOT>/PRD.md

## 禁止

- 不要设计技术方案（那是 SPEC 的事）
- 不要拆解任务（那是 TASK 的事）
- 不要阅读任何已有代码`
})
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| 用户对项目的描述原文 | 之前对话中的非需求讨论 |
| agents/prd-agent.md 内容 | 已有代码内容 |
| BRAINSTORM.md 内容（如存在） | 其他阶段的文档（SPEC/TASK） |
| DOC_ROOT 路径 | 会话历史或 session 记录 |

## 完成后

1. 确认 PRD.md 已写入
2. 更新 STATUS.md：当前阶段 → PRD
3. 提示用户：确认 PRD 后执行 `/spec` 推进
