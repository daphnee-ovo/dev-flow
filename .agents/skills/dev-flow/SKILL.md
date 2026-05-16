---
name: dev-flow
description: "项目全流程管理。命令：/init（项目初始化）、/brainstorm（头脑风暴）、/prd（需求探索）、/spec（技术规范）、/task（任务拆解）、/devtest（例行测试）、/fix（自动修复 issue）、/test（完整测试）、/done（交付确认）、/status（状态报告）、/check（文档同步检查）、/iterate（启动新迭代）、/mode（开发模式）。当用户提到创建项目、启动项目、初始化、项目状态、下一步、开始开发、新版本、迭代、头脑风暴、想法、模式时触发。"
---

# Dev-Flow：项目全流程管理

## Skill 加载确认

当此 skill 被触发时，输出：

```
[dev-flow] skill 已加载 | 当前阶段：<从 STATUS.md 读取，不存在则显示"新项目">
```

## 流程总览

```
需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 交付(DONE)
                                       │                ↑
                                       └── 例行TEST ──→│
```

## 命令映射

| 命令 | 阶段 | 角色 |
|------|------|------|
| `/init` | 初始化 | 创建 dev-doc、选择模式 |
| `/brainstorm` | PRD 前置 | 协作式设计探索 |
| `/prd` | PRD | 懂技术的高级产品经理 |
| `/spec` | SPEC | 资深架构师 |
| `/task` | TASK | 经验丰富的技术主管 |
| `/devtest` | DEV（内循环） | 轻量 QA |
| `/fix` | DEV/TEST | 自动修复 issue |
| `/test` | TEST | 严格的 QA 工程师 |
| `/done` | DONE | 项目经理 |
| `/status` | 任意 | 状态报告 |
| `/check` | 任意 | 文档同步检查 |
| `/iterate` | DONE → 新轮次 | 归档 + 重置 |
| `/mode` | 任意 | 模式选择（full/quick/fast/mvp） |

## 角色隔离

不同阶段由独立 agent 执行，避免上下文互相干扰。每个 agent 只接收该阶段所需的最小输入。

| 阶段 | 执行方式 | 输入 |
|------|----------|------|
| PRD | 独立 agent | 项目基本信息 |
| SPEC | 独立 agent | PRD.md |
| TASK | 独立 agent | SPEC.md |
| DEV | 主 agent 直接执行 | TASK.md + SPEC.md |
| TEST | 独立 agent | SPEC.md + TASK.md |
| DONE | 主 agent 直接执行 | 全部文档 |

## DEV 阶段规则

开发阶段由主 agent 执行，遵循：
- 只做 TASK.md 列出的任务，不多不少
- 完成一个任务立即勾选，立即触发 `/devtest`
- 文档实时更新，不允许"稍后再改"
- 所有任务完成后自动进入 `/test`

## 目录结构

```
dev-doc/
├── STATUS.md
├── BRAINSTORM.md
├── PRD.md
├── SPEC.md
├── TASK.md
├── TEST.md
├── issue/
│   ├── issue_test_2026-05-15_1.md           # 未关闭
│   └── closed_issue_test_2026-05-14_1.md    # 已关闭
├── session/
│   ├── 001-init-project.md
│   └── memory/
└── archive/
    └── v1/
```

## 灵活性

- 小项目可合并阶段（如 PRD+SPEC 一步）
- 用户明确知道要什么时，不强制走完整流程
- 流程服务于项目，不是项目服务于流程
