# Dev-Flow Plugin

项目全流程管理插件，从需求探索到交付的完整生命周期管理。

## 命令

| 命令 | 作用 |
|------|------|
| `/prd` | 启动 PRD agent，进入需求探索阶段 |
| `/spec` | 启动 SPEC agent，进入技术规范阶段 |
| `/task` | 启动 TASK agent，进入任务拆解阶段 |
| `/dev-test` | 开发中例行测试（任务级验证） |
| `/test` | 启动完整 TEST agent（项目级全量验证） |
| `/done` | 执行交付检查 |
| `/status` | 报告当前项目状态和进度 |

## 流程

```
需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 交付(DONE)
                                       │                ↑
                                       │  例行TEST      │
                                       │  (任务级循环)    │  项目TEST
                                       └───────────────→│  (全量验证)
```

## Hooks

- `PostToolUse(Write|Edit)`: 自动更新 STATUS.md、检查阶段完成标准
- `Stop`: 会话结束时保存记录

## 目录结构约定

项目使用 `dev-doc/` 目录管理所有流程文档。
