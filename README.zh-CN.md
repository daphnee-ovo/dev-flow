**语言:** [English](README.md) | 中文

---

<div align="center">

# dev-flow

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/network)

**给 Coding Agent 的工程纪律**

小而美，不追求大而全。用轻量文档、规范阶段和硬约束，把 coding agent 的原始编码能力转化为可靠的工程交付。

</div>

## 快速开始

### 一条命令安装

```bash
# macOS arm64 / Linux x86_64 / Linux aarch64
brew install daphnee-ovo/tap/dev-flow

# Linux / macOS / WSL
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
```

Homebrew 目前支持 macOS arm64、Linux x86_64 和 Linux aarch64。安装脚本支持 Linux、macOS、WSL 和 Windows。setup 会把 dev-flow 注册到对应 agent（Claude Code、Codex 或两者）。具体项目初始化是在目标项目里执行 `/init`。

### 基本流程

```bash
cd your-project
```

然后对 coding agent 输入：

```text
/init
/task
```

dev-flow 会创建 `.dev-doc/` 工作区，在 `STATUS.yaml` 里记录当前阶段，生成结构化 task 文件，并通过 hooks 在 agent 违反流程规则时提醒或拦截。

完整示例见 [examples/quickstart-demo.md](examples/quickstart-demo.md)。

如果用 Homebrew 安装，使用 `/init` 前先运行一次 `dow setup`。

---

## 为什么需要 dev-flow

Coding agent 很会改代码，但长任务里容易丢需求、跳验证、忘同步文档，或者把实现状态和交付状态混在一起。dev-flow 给 Claude Code 和 Codex CLI 加一层轻量工作流：

- 改代码前先明确需求和边界
- 把设计、任务拆解、实现、QA 拆成明确阶段
- 让 `.dev-doc/` 和真实项目状态保持同步
- 用 hooks 阻止 agent 跳过检查、写入不安全临时目录、丢失 changelog 上下文
- 每次交付都归档，后续迭代可以追溯

适合 feature、重构、审计、多步修复这类需要工程纪律的 agent 任务。

## 不适合的场景

dev-flow 是有明确取舍的工具。单行修改、临时脚本、不希望仓库里出现流程文件的项目，不一定需要它。当 agent 跑偏的成本高于轻量流程成本时，它才值得引入。

---

## 支持的 Agent

| Agent | 状态 | 手动 setup |
|-------|------|------|
| **Claude Code** | 已支持 | `dow setup --agent claude` |
| **Codex CLI** | 已支持 | `dow setup --agent codex` |
| **Kiro CLI** | 测试中 | `dow setup --agent kiro` |

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

---

## 命令

| 命令 | 说明 |
|------|------|
| `/init` | 初始化项目（创建 .dev-doc、选择模式、规范校验） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动 PRD agent，产出 PRD.md |
| `/spec` | 启动 SPEC agent，产出 SPEC.md |
| `/task` | 启动 TASK agent，产出任务文件 |
| `/issue` | 手动创建 issue 文件 |
| `/devtest` | 开发中例行测试（任务级验证） |
| `/fix` | 自动读取未关闭 issue 并修复 |
| `/test` | 完整 TEST agent（项目级全量验证） |
| `/status` | 查看当前项目状态和进度 |
| `/check` | 检查开发工作是否已同步到 .dev-doc |
| `/iterate` | 交付后启动新迭代（归档 + 重置） |
| `/mode` | 选择开发模式（full/quick/fast/mvp；audit 为自动触发） |

---

## 开发模式

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | prd → spec → task → dev → test → iterate | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → iterate | 需求明确的功能开发 |
| `fast` | task → dev → test → iterate | 小改动、技术方案已知 |
| `mvp` | spec → task → dev → iterate | 快速验证，跳过 TEST |

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

插件在项目中维护 `.dev-doc/` 目录：

```
.dev-doc/
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

dev-flow 同时支持 **Claude Code** 和 **OpenAI Codex CLI**，通过共享插件核心 + 各 agent 适配层实现：

| 组件 | Claude Code | Codex CLI |
|------|-------------|-----------|
| 插件 manifest | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| Hooks 配置 | `hooks/hooks.json` | `hooks.json`（根目录） |
| 项目指令 | `CLAUDE.md` | `AGENTS.md` |
| 子代理 API | `Agent({...})` | `spawn_agent` |

命令、skills 和 agents 跨平台共享。Hooks 直接调用全局 `dow` CLI。

### dow CLI

`dow` 是统一调度器，驱动所有 hooks 和自动化：

| 命令 | 说明 |
|------|------|
| `dow setup [--agent claude\|codex\|all]` | 注册插件到 agent（交互式 TUI） |
| `dow update` | 自更新二进制 + 插件 |
| `dow self-check` | 查看安装状态 |
| `dow status` | 读写 STATUS.yaml |
| `dow iterate` | 交付：归档 + commit + tag + bump |
| `dow doc <type>` | 生成/查询文档模板 |
| `dow hooks ...` | Hook 调度（context, guard, post-write） |

---

## 项目结构

```
dev-flow/
├── dow/                        # Rust CLI 源码（dow 二进制）
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli.rs
│   │   ├── commands/           # 子命令实现
│   │   │   ├── setup.rs        # dow setup
│   │   │   ├── update.rs       # dow update
│   │   │   └── self_check.rs   # dow self-check
│   │   ├── hooks/              # Hook 实现
│   │   └── core/               # 公共库
│   │       ├── config.rs       # ~/.config/dow/config.toml
│   │       ├── platform.rs     # XDG 路径、平台检测
│   │       ├── github.rs       # Release API、自更新
│   │       └── agent_registry.rs # 插件部署
│   └── Cargo.toml
├── plugin/                     # 共享插件内容（agent 无关）
│   ├── skills/
│   ├── commands/
│   └── agents/
├── targets/                    # 各 agent 适配层
│   ├── claude/
│   │   ├── plugin.json
│   │   └── hooks.json
│   └── codex/
│       ├── plugin.json
│       └── hooks.json
├── install/                    # 一条命令安装脚本
│   ├── install.sh              # curl | bash
│   └── install.ps1             # irm | iex
├── examples/                   # 快速开始和流程示例
├── devtools/                   # 开发辅助
│   ├── assemble.sh             # 组装 dist/<agent>/
│   └── deploy-local.sh         # 编译 + 本地部署
├── .github/workflows/
│   └── release.yml             # CI：tag → 构建 → GitHub Release
├── VERSION
├── CLAUDE.md
├── AGENTS.md
├── README.md
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
