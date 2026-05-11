# dev-flow-plugin

项目全流程管理插件 for Claude Code。从需求探索到交付的完整生命周期管理。

## 安装

```bash
# 本地安装（开发测试）
claude /plugin install --local /path/to/dev-flow-plugin

# 从 marketplace 安装（发布后）
claude /plugin install dev-flow
```

## 命令

| 命令 | 说明 |
|------|------|
| `/prd` | 启动需求探索，产出 PRD.md |
| `/spec` | 启动技术规范设计，产出 SPEC.md |
| `/task` | 启动任务拆解，产出 TASK.md |
| `/dev-test` | 开发中例行测试（任务级验证） |
| `/test` | 项目级全量测试 |
| `/done` | 交付确认检查 |
| `/status` | 查看当前项目状态 |

## 流程

```
PRD → SPEC → TASK → DEV → TEST → DONE
                     │         ↑
                     └─ /dev-test (循环) ─→│
```

每个阶段有独立的 agent 角色，保证思维模式隔离：
- PRD: 懂技术的高级产品经理
- SPEC: 资深架构师
- TASK: 经验丰富的技术主管
- DEV: 老练的程序员（主 agent 直接执行）
- TEST: 严格的 QA 工程师
- DONE: 项目经理

## Hooks

插件包含 3 个自动化 hooks：

1. **update-status** — 文档变更时自动更新 STATUS.md
2. **check-phase-completion** — 阶段文档写完后检查完成标准
3. **save-session** — 会话结束时保存记录

## 目录结构

插件会在项目中创建 `dev-doc/` 目录管理流程文档：

```
dev-doc/
├── STATUS.md          # 项目状态
├── PRD.md             # 产品需求
├── SPEC.md            # 技术规范
├── TASK.md            # 任务清单
├── TEST.md            # 测试文档
├── issue/             # 问题追踪
└── session/           # 会话记录
    ├── task/
    └── memory/
```

支持多工程模式（按 git 分支隔离）：`dev-doc/<branch_name>/`

## License

MIT
