# Dev-Flow Plugin

项目全流程管理插件，从需求探索到交付的完整生命周期管理。

## 命令

| 命令 | 作用 |
|------|------|
| `/init` | 初始化 dev-flow 项目（创建 dev-doc、选择模式） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动 PRD agent，进入需求探索阶段 |
| `/spec` | 启动 SPEC agent，进入技术规范阶段 |
| `/task` | 启动 TASK agent，进入任务拆解阶段 |
| `/issue` | 手动创建 issue 文件 |
| `/devtest` | 开发中例行测试（任务级验证） |
| `/fix` | 自动读取未关闭 issue 并修复 |
| `/test` | 启动完整 TEST agent（项目级全量验证） |
| `/status` | 报告当前项目状态和进度 |
| `/check` | 检查开发工作是否已同步到 dev-doc |
| `/iterate` | 交付后启动新迭代（归档 + 重置） |
| `/mode` | 选择开发模式（full/quick/fast/mvp） |

## 流程

```
头脑风暴(BRAINSTORM) → 需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 交付(DONE)
        可选                                                  │                ↑         │
                                                              │  例行TEST      │         │
                                                              │  (任务级循环)    │ 项目TEST │
                                                              └───────────────→│         │
                                                                                         ↓
                                                                              迭代(ITERATE) → 下一轮
```

## Hooks

- `PostToolUse(Write|Edit)`: 自动更新 STATUS.yaml、检查阶段完成标准
- `Stop`: 会话结束时保存记录

## Codex 兼容

- Codex 插件入口：`.codex-plugin/plugin.json`
- Codex skill 入口：`skills/dev-flow/SKILL.md`
- Codex hooks 入口：`hooks.json`（使用相对路径调用 `scripts/hooks/`）
- 命令中要求独立 agent 时，Codex 使用 `spawn_agent`，Claude Code 使用 `Agent`
- `/init` 更新项目级指令时，Codex 优先写 `AGENTS.md`，Claude Code 优先写 `CLAUDE.md`

## 目录结构约定

项目使用 `dev-doc/` 目录管理所有流程文档。

