**语言:** [English](README.md) | 中文

---

<div align="center">

# dev-flow

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/network)

**Claude Code & Codex CLI 的轻量工程管理插件**

小而美，不追求大而全。先想清楚再动手，用轻量文档、规范阶段和硬约束，让 agent 按工程管理流程工作。

</div>

---

## 项目理念

dev-flow 的核心不是堆叠更多流程、角色和文档，而是在保持轻量的前提下，帮助 agent 先理清想法、再进入实现，并形成足够强的工程约束。

关键词：

- **先想清楚再做** — 有想法先梳理目标、边界、方案和验收标准，再进入实现，避免上来就改代码。
- **轻量** — 文档和命令只保留能推动交付的部分，避免为了流程而流程。
- **规范** — PRD、SPEC、TASK、TEST、issue、archive 都有稳定结构，便于追踪和复用。
- **约束** — 通过阶段、hooks、检查和任务闭环，阻止 agent 跳过需求、规范、验证和交付门禁。
- **目标必要性** — 每个能力都要回答“它是否服务于当前目标”。必要的约束必须保留；不必要的仪式不能引入。
- **同步性** — 流程文档必须和真实项目状态同步，包括代码、任务、版本、测试和迭代。管理文档一旦脱离实际进度，就会从帮助变成噪音。
- **模式适配** — 快速验证和长期工程不是同一种流程。MVP 可以先跑通、测明显问题；标准开发可以再提高测试、review 和发布门禁。

## 快速开始

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

在 Codex 中打开 `/plugins`，搜索 `Dev-Flow` 并安装。安装后执行 `/init` 初始化项目。

> 本地开发时也可以直接添加当前目录：
> ```bash
> codex plugin marketplace add .
> ```

### 基本流程

```
/init          → 初始化项目，选择开发模式
/brainstorm    → 协作式需求探索（可选）
/prd           → 产出需求文档
/spec          → 产出技术规范
/task          → 拆解任务清单
               → 开发（完成任务后自动触发 /devtest）
/test          → 全量测试

```

---

## 命令

| 命令 | 说明 |
|------|------|
| `/init` | 初始化项目（创建 dev-doc、选择模式、规范校验） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动 PRD agent，产出 PRD.md |
| `/spec` | 启动 SPEC agent，产出 SPEC.md |
| `/task` | 启动 TASK agent，产出任务文件 |
| `/issue` | 手动创建 issue 文件 |
| `/devtest` | 开发中例行测试（任务级验证） |
| `/fix` | 自动读取未关闭 issue 并修复 |
| `/test` | 完整 TEST agent（项目级全量验证） |
| `/status` | 查看当前项目状态和进度 |
| `/check` | 检查开发工作是否已同步到 dev-doc |
| `/iterate` | 交付后启动新迭代（归档 + 重置） |
| `/mode` | 选择开发模式（full/quick/fast/mvp；audit 为自动触发） |

---

## 开发模式

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → iterate | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → iterate | 需求明确的功能开发 |
| `fast` | task → dev → test → iterate | 小改动、技术方案已知 |
| `mvp` | brainstorm → spec → dev | 快速验证想法、原型 |

> `audit` 模式在非 DEV 阶段创建 issue 时自动触发。格式：`audit/<原模式>`。iterate 后自动恢复。

---

## 核心特性

### 角色隔离

每个阶段由独立 agent 执行，避免思维定势：

| 阶段 | 角色 |
|------|------|
| PRD | 懂技术的高级产品经理 |
| SPEC | 资深架构师 |
| TASK | 经验丰富的技术主管 |
| DEV | 主 agent 直接执行 |
| TEST | 严格的 QA 工程师 |

### 自动化 Hooks

无需手动操作：

- **上下文注入** — 每次对话注入当前阶段状态和规范提醒
- **自动 devtest** — 任务完成后自动触发例行测试
- **文档同步检查** — 代码变更时提醒同步文档
- **变更日志** — 会话结束时自动保存 CHANGELOG
- **系统临时目录拦截** — 禁止写入系统临时目录；项目内 `tmp` 和 `temp` 都允许，新项目默认使用 `tmp`

### 文档驱动开发

插件在项目中维护 `dev-doc/` 目录：

```
dev-doc/
├── STATUS.yaml            # 项目状态
├── CHANGELOG.md           # 会话变更日志（追加式）
├── BRAINSTORM.md          # 头脑风暴
├── PRD.md                 # 产品需求
├── SPEC.md                # 技术规范
├── TEST.md                # 测试报告
├── task/                  # 任务文件（task_<日期>_<序号>.md）
├── issue/                 # 问题追踪（issue_<来源>_<日期>_<序号>.md）
└── archive/               # 历史迭代（v<N>-<主题>/）
```

### 迭代管理

`/iterate` 归档当前版本，启动新一轮开发。所有文档会归档到 `archive/v<N>-<主题>/` 下。

---

## 跨平台支持

dev-flow 同时支持 **Claude Code** 和 **OpenAI Codex CLI**：

| 组件 | Claude Code | Codex CLI |
|------|-------------|-----------|
| 插件 manifest | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| Skill 入口 | `.claude/skills/dev-flow/SKILL.md` | `skills/dev-flow/SKILL.md` |
| Hooks 配置 | `hooks/hooks.json` | `hooks.json`（根目录） |
| 项目指令 | `CLAUDE.md` | `AGENTS.md` |
| 子代理 API | `Agent({...})` | `spawn_agent` |

`commands/` 中的命令以运行时中立的方式编写，两个平台通用。

---

## 项目结构

```
dev-flow/
├── .claude-plugin/
│   ├── plugin.json            # Claude Code 插件配置
│   └── marketplace.json       # Marketplace 元数据
├── .codex-plugin/
│   └── plugin.json            # Codex CLI 插件 manifest
├── .claude/skills/dev-flow/
│   └── SKILL.md               # Claude Code skill 触发
├── skills/dev-flow/
│   └── SKILL.md               # Codex CLI skill 入口
├── commands/                   # Slash 命令定义
│   ├── init.md
│   ├── brainstorm.md
│   ├── prd.md
│   ├── spec.md
│   ├── task.md
│   ├── issue.md
│   ├── devtest.md
│   ├── fix.md
│   ├── test.md
│   ├── done.md
│   ├── status.md
│   ├── check.md
│   ├── iterate.md
│   └── mode.md
├── agents/                     # Agent prompt 模板
│   ├── prd-agent.md
│   ├── spec-agent.md
│   ├── task-agent.md
│   └── test-agent.md
├── hooks/
│   └── hooks.json              # Claude Code hook 注册
├── hooks.json                  # Codex CLI hook 注册
├── scripts/
│   ├── hooks/                  # Hook 脚本
│   │   ├── inject-context.sh
│   │   ├── block-system-tmp.sh
│   │   ├── block-non-dev-edit.sh
│   │   ├── post-write.sh
│   │   └── save-changelog.sh
│   ├── commands/               # 脚本化命令
│   │   ├── devtest.sh
│   │   ├── status.sh
│   │   ├── check.sh
│   │   ├── mode.sh
│   │   └── iterate.sh
│   └── init/                   # Init 命令脚本
│       ├── scan-project.sh
│       ├── validate.sh
│       └── migrate.sh
├── references/                 # 唯一格式 schema 层（权威定义）
│   ├── dev-flow-spec.md
│   └── dev-doc/                # 文档格式 schema（权威）
│       ├── BRAINSTORM-FILE.md
│       ├── CHANGELOG.md
│       ├── ISSUE.md
│       ├── PRD-FILE.md
│       ├── SPEC-FILE.md
│       ├── STATUS.md
│       ├── STATUS.yaml
│       ├── TASK-FILE.md
│       └── TEST.md
├── CLAUDE.md
├── AGENTS.md
├── README.md
├── README.zh-CN.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## 贡献

参见 [CONTRIBUTING.md](CONTRIBUTING.md) 了解本地开发和约定。

---

## 致谢

`/brainstorm` 命令灵感来自 [superpowers](https://github.com/obra/superpowers)。

---

## License

[MIT](LICENSE)
