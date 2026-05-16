# dev-flow

项目全流程管理插件 for Claude Code。从需求探索到交付的完整生命周期管理。

## 安装

```bash
# 本地安装（开发测试）
claude /plugin install --local /path/to/dev-flow

# 从 marketplace 安装（发布后）
claude /plugin install dev-flow
```

## 命令

| 命令 | 说明 |
|------|------|
| `/init` | 初始化项目（创建 dev-doc、选择模式、规范校验） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动需求探索，产出 PRD.md |
| `/spec` | 启动技术规范设计，产出 SPEC.md |
| `/task` | 启动任务拆解，产出 TASK.md |
| `/devtest` | 开发中例行测试（任务级验证） |
| `/fix` | 自动读取未关闭 issue 并修复 |
| `/test` | 项目级全量测试 |
| `/done` | 交付确认检查 |
| `/status` | 查看当前项目状态 |
| `/check` | 文档同步检查 |
| `/iterate` | 交付后启动新迭代 |
| `/mode` | 选择开发模式（full/quick/fast/mvp） |

## 开发模式

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → done | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → done | 需求明确的功能开发 |
| `fast` | task → dev → test → done | 小改动、技术方案已知 |
| `mvp` | brainstorm → spec → dev | 快速验证想法、原型 |

## 流程

```
头脑风暴(BRAINSTORM) → 需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 交付(DONE)
                                                               │                ↑
                                                               └── /devtest ──→ │
```

每个阶段有独立的 agent 角色，保证思维模式隔离：
- PRD: 懂技术的高级产品经理
- SPEC: 资深架构师
- TASK: 经验丰富的技术主管
- DEV: 老练的程序员（主 agent 直接执行）
- TEST: 严格的 QA 工程师
- DONE: 项目经理

## Hooks

插件包含自动化 hooks：

1. **inject-context** — 每次对话注入阶段状态和规范提醒
2. **block-system-tmp** — 禁止使用系统 /tmp/，强制项目内 tmp/
3. **check-task-completion** — 任务勾选后自动触发 /devtest
4. **check-doc-sync** — 代码变更时提醒同步文档
5. **check-phase-completion** — 阶段文档完成标准检查
6. **update-status** — 文档变更时自动更新 STATUS.md
7. **save-session** — 会话结束时保存记录

## 目录结构

插件会在项目中创建 `dev-doc/` 目录管理流程文档：

```
dev-doc/
├── STATUS.md          # 项目状态
├── BRAINSTORM.md      # 头脑风暴（持久，不归档）
├── PRD.md             # 产品需求
├── SPEC.md            # 技术规范
├── TASK.md            # 任务清单
├── TEST.md            # 测试报告
├── issue/             # 问题追踪
│   ├── issue_<source>_<date>_<seq>.md
│   └── closed_issue_<source>_<date>_<seq>.md
├── session/           # 会话记录
│   ├── <seq>-<topic>.md
│   └── memory/
└── archive/           # 历史迭代
    └── v<N>/
```

支持多工程模式（按 git 分支隔离）：`dev-doc/<branch_name>/`

## License

MIT
