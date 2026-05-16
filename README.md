# dev-flow

项目全流程管理插件，支持 Claude Code 和 OpenAI Codex CLI。从需求探索到交付的完整生命周期管理。

## 安装

### Claude Code

```bash
# 添加 marketplace
/plugin marketplace add daphnee-ovo/dev-flow

# 安装插件
/plugin install dev-flow@daphnee-ovo
```

### Codex CLI

```bash
# 添加 marketplace
codex plugin marketplace add daphnee-ovo/dev-flow
```

然后在 Codex 中打开 `/plugins`，搜索 `Dev-Flow` 并安装。安装后执行 `/init` 初始化项目。

本地开发时也可以直接添加当前目录：

```bash
codex plugin marketplace add .
```

## 快速开始

```
/init          → 初始化项目，选择开发模式
/brainstorm    → 协作式需求探索（可选）
/prd           → 产出需求文档
/spec          → 产出技术规范
/task          → 拆解任务清单
               → 开发（完成任务后自动触发 /devtest）
/test          → 全量测试
/done          → 交付确认
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
| `/mode` | 选择开发模式 |

## 开发模式

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → done | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → done | 需求明确的功能开发 |
| `fast` | task → dev → test → done | 小改动、技术方案已知 |
| `mvp` | brainstorm → spec → dev | 快速验证想法、原型 |

## 核心特性

**角色隔离** — 每个阶段由独立 agent 执行，避免思维定势：

| 阶段 | 角色 |
|------|------|
| PRD | 懂技术的高级产品经理 |
| SPEC | 资深架构师 |
| TASK | 经验丰富的技术主管 |
| DEV | 主 agent 直接执行 |
| TEST | 严格的 QA 工程师 |

**自动化 hooks** — 无需手动操作：
- 每次对话注入当前阶段状态和规范提醒
- 任务完成后自动触发例行测试
- 代码变更时提醒同步文档
- 会话结束时自动保存记录

Codex 使用 `.codex-plugin/plugin.json`、`skills/dev-flow/SKILL.md` 和根目录 `hooks.json` 加载插件；Claude Code 继续使用 `.claude-plugin/`、`.claude/skills/` 和 `hooks/hooks.json`。

**文档驱动** — 插件在项目中维护 `dev-doc/` 目录：

```
dev-doc/
├── STATUS.md              # 项目状态
├── BRAINSTORM.md          # 头脑风暴
├── PRD.md                 # 产品需求
├── SPEC.md                # 技术规范
├── TASK.md                # 任务清单
├── TEST.md                # 测试报告
├── issue/                 # 问题追踪
├── session/               # 会话记录
└── archive/               # 历史迭代
```

**迭代管理** — `/iterate` 归档当前版本，启动新一轮开发。

## 致谢

`/brainstorm` 命令灵感来自 [superpowers](https://github.com/obra/superpowers)。

## License

MIT
