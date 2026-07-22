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
# Cargo（需要 Rust 工具链）
cargo install dev-flow && dow setup

# macOS arm64 / Linux x86_64 / Linux aarch64
brew install daphnee-ovo/tap/dev-flow && dow setup

# Linux / macOS / WSL
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
```

安装脚本会自动运行 `dow setup`。Cargo 和 Homebrew 安装后需手动执行 `dow setup`，将 dev-flow 注册到对应 agent（Claude Code、Codex 或 Kiro）。具体项目初始化是在目标项目里执行 `/init`。

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

完整示例见 [examples/quickstart-demo.md](examples/quickstart-demo.md)，也可以直接查看 [examples/sample-project](examples/sample-project/) 里的静态 `.dev-doc` 产物。

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
| **Codex** | 已支持 | `dow setup --agent codex` |
| **Kiro-cli** | 已支持 | `dow setup --agent kiro` |

### Agent 兼容性

三个 agent 提供完全一致的工作流体验 — 命令、hooks、子 agent、状态管理均相同。唯一差异是平台实现方式：

| 方面 | Claude Code | Codex CLI / App | Kiro |
|------|-------------|-----------------|------|
| 命令接口 | Slash commands | Skill commands | Skill commands |
| 子 agent 调用 | `Agent` tool | `spawn_agent` | subagent |
| 项目指令文件 | `CLAUDE.md` | `AGENTS.md` | `.kiro/steering/` |

#### Kiro：启用 Hooks

Kiro 的默认 agent 不支持 hook 配置。安装后需将 dev-flow agent 设为默认：

```bash
kiro-cli agent set-default --name dev-flow
```

`dow setup --agent kiro` 会在注册完成后提醒此步骤。不执行的话 hooks 不会触发。

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
| `/prd` | PRD 阶段 — 主 agent 产出 PRD.md，审计 agent 审核 |
| `/spec` | SPEC 阶段 — 主 agent 产出 SPEC.md，审计 agent 审核 |
| `/task` | TASK 阶段 — 拆解任务文件（复杂情况由 challenger agent 辅助） |
| `/issue` | 手动创建 issue 文件 |
| `/test` | 执行 dow test 全量测试 |
| `/fix` | 用户显式触发：读取、认领、修复、验证并关闭未关闭 issue |
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

主 agent 驱动各阶段，独立审计/挑战子 agent 审核产出：

| 阶段 | 执行 | 审核 |
|------|------|------|
| BRAINSTORM | 主 agent | brainstorm-audit-agent |
| PRD | 主 agent | prd-audit-agent |
| SPEC | 主 agent | spec-audit-agent |
| TASK | 主 agent | task-challenger-agent（复杂情况） |
| DEV | 主 agent | — |
| TEST | `dow test` CLI | test-agent（失败分析） |

### 自动化 Hooks

无需手动操作：

- **上下文注入** — 每次对话注入当前阶段状态和规范提醒
- **Task 关闭测试门禁** — dow task done TASK-ID 在改写 Task 前先执行 dow test TASK-ID
- **文档同步检查** — 代码变更时提醒同步文档
- **变更日志** — 会话结束时自动保存 CHANGELOG
- **系统临时目录拦截** — 禁止写入系统临时目录；项目内 `tmp` 和 `temp` 都允许，新项目默认使用 `tmp`

### 文档驱动开发

插件在项目中维护 `.dev-doc/` 目录，按分支组织：

```
.dev-doc/
├── archive.db             # SQLite 归档，通过 `dow archive ...` 查询
├── preIterate.ci          # 可选的迭代前 CI 步骤
└── <分支名>/              # 当前分支流程文档（main/beta/...）
    ├── STATUS.yaml        # 项目状态
    ├── CHANGELOG.md       # 会话变更日志（追加式）
    ├── BRAINSTORM.md      # 头脑风暴
    ├── PRD.md             # 产品需求
    ├── SPEC.md            # 技术规范
    ├── task/              # 任务文件（task_<日期>_<序号>.md）
    └── issue/             # 问题追踪（issue_<来源>_<日期>_<序号>.md）
```

### 迭代管理

`/iterate` 会把已完成 task、已关闭 issue、测试报告、CHANGELOG 和阶段文档写入 `.dev-doc/archive.db`，然后启动新一轮开发。历史迭代通过 `dow archive list/show/tasks/issues/doc` 查询。

如果存在 `.dev-doc/preIterate.ci`，`dow iterate --confirm` 会先执行其中的 steps，再归档、commit、tag、bump。任一步失败都会阻断整个 iterate。支持 `sync-version: <path>` 同步显式声明的 Cargo/npm/uv 清单版本，也支持 `run: <command>` 执行项目内检查、lockfile 更新或生成命令。

```text
run: bash tests/test_all.sh
sync-version: dow/Cargo.toml
sync-version: npm/dev-flow/package.json
run: cargo update -p dev-flow --manifest-path dow/Cargo.toml
```

### Web 看板与依赖图

`dow dashboard` 启动本地 web 看板，包含：

- **看板视图** — 任务按 In Progress / Pending / Done 分组，issue 按 In Progress / Open / Closed 分组
- **依赖图** — 使用 D3 force simulation 可视化显式和隐式任务/issue 依赖。隐式依赖基于任务间文件交集推断。进行中的节点闪烁。
- **文档查看器** — 内联浏览 BRAINSTORM、PRD、SPEC 文档
- **筛选** — 按优先级（P0/P1/P2）和状态过滤
- **状态概览** — 当前阶段、模式和迭代状态

### Claim 认领系统

`dow claim` 让 agent 在开始工作前认领 task 或 issue：

- **依赖检查** — 上游依赖未解决时阻止认领
- **文件范围保护** — guard hook 在写入声明文件之外时发出警告
- **认领锁** — 存储在 `.dev-doc/<分支>/claim.lock`，防止并发认领
- **In Progress 可见性** — 已认领在看板的 In Progress 列显示

### Issue 跟踪

Issue 支持完整的生命周期：

- **嵌套文件范围**：create/update 使用 `--file '{"create":[],"modify":["src/a.rs"]}'`；stdin JSON 使用顶层 `files` 对象。`create`、`modify` 可分别省略，但至少一个必须包含非空路径。
- **JSON 批量创建**：支持单个嵌套 JSON 对象或 JSON 数组。
- **多行值**：description/reproduce/fix 支持 YAML 缩进续行格式
- **关闭强制**：关闭时必须填写非空 fix 字段
- **增量文件更新**：`dow issue update I001 --file '{"modify":["+src/foo.rs","-src/bar.rs"]}'`
- **输出契约**：JSON detail 输出使用嵌套 `files`；issue Markdown 继续保持 `files_modify`/`files_create` 格式。
- **修复流程**：只有用户显式调用 `/fix` 后才运行。它会读取并认领未关闭 issue，执行范围明确的修复，使用 `dow issue update --fix` 记录结果，完成验证后使用 `dow issue close` 关闭 issue。

### 多分支 VERSION

`VERSION` 文件支持各分支独立版本管理：

```
(main)0.2.4
(beta)0.3.5
```

`build.rs` 通过 `git rev-parse` 检测当前分支，编译时选择对应的版本行。`dow version` 和编译后的二进制都返回分支特有版本。

---

## 跨平台支持

dev-flow 同时支持 **Claude Code** 和 **OpenAI Codex CLI**，通过共享插件核心 + 各 agent 适配层实现：

| 组件 | Claude Code | Codex CLI |
|------|-------------|-----------|
| 插件 manifest | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| Hooks 配置 | `targets/claude/hooks.json` | `targets/codex/hooks.json` |
| 项目指令 | `CLAUDE.md` | `AGENTS.md` |
| 子代理 API | `Agent({...})` | `spawn_agent` |

命令、skills 和 agents 跨平台共享。Hooks 直接调用全局 `dow` CLI。

### dow CLI

`dow` 是统一调度器，驱动所有 hooks 和自动化：

| 命令 | 说明 |
|------|------|
| `dow setup [--agent claude\|codex\|all]` | 注册插件到 agent（交互式 TUI） |
| `dow update` | 自更新二进制 + 插件 |
| `dow doctor [--fix]` | 诊断 .dev-doc 结构和规范一致性 |
| `dow status` | 读写 STATUS.yaml |
| `dow claim <TASK-ID\|ISSUE-ID>` | 认领 task 或 issue（含依赖检查） |
| `dow task create/update/show/list` | Task 全生命周期管理 |
| `dow issue create/update/close/show/list` | Issue 全生命周期管理 |
| `dow fix` | `dow doctor --fix` 的兼容别名 |
| `dow test` | 项目级全量测试 |
| `dow test <TASK-ID>` | 执行 Task 的 files.test |
| `dow scan` | 项目结构扫描 |
| `dow version [--set X.Y.Z] [--bump patch]` | 读写多分支 VERSION |
| `dow iterate [--confirm]` | 交付：归档 + commit + tag + bump |
| `dow rollback --version <v>` | 回滚迭代：从归档恢复任务/issue/文档 |
| `dow task/issue/prd/spec/brainstorm/changelog schema` | 获取对应文档 schema |
| `dow dashboard [--port PORT] [--no-open]` | 启动本地 web 看板（依赖图 + 看板 + 文档） |
| `dow hooks ...` | Hook 调度（context, guard, post-write） |
| `dow archive list/show/tasks/issues/doc` | 从 archive.db 查询历史迭代 |

---

## VS Code 插件

**Dow Dashboard** 插件将 dev-flow 的 dashboard 以 webview 面板形式嵌入 VS Code。

### 安装

```bash
cd vscode-extension
npm install
npm run compile
```

按 `F5` 启动扩展开发宿主，或打包安装：

```bash
npx vsce package
code --install-extension dow-dashboard-0.1.0.vsix
```

### 使用

打开命令面板（`Ctrl+Shift+P` / `Cmd+Shift+P`），执行：

```
Dow: Open Dashboard
```

Dashboard 展示项目的任务/Issue 依赖图、看板、文档查看器和状态概览——与 `dow dashboard` 提供的内容一致，但集成在编辑器中。

### 前置条件

- `dow` CLI 已安装且在 PATH 中
- 工作区中存在 `.dev-doc/` 目录（先执行 `/init`）

---

## 项目结构

```
dev-flow/
├── dow/                           # Rust CLI 源码（dow 二进制）
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli.rs
│   │   ├── commands/              # 24 个子命令模块
│   │   │   ├── setup.rs          # dow setup
│   │   │   ├── doctor.rs         # dow doctor
│   │   │   ├── claim.rs          # dow claim
│   │   │   ├── dashboard.rs      # dow dashboard
│   │   │   ├── issue.rs          # dow issue
│   │   │   ├── task.rs           # dow task
│   │   │   ├── iterate.rs        # dow iterate
│   │   │   ├── rollback.rs       # dow rollback
│   │   │   ├── version.rs        # dow version
│   │   │   └── ...
│   │   ├── hooks/                # Hook 实现
│   │   │   ├── context.rs
│   │   │   ├── guard.rs
│   │   │   ├── post_write.rs
│   │   │   ├── post_bash.rs
│   │   │   └── save_changelog.rs
│   │   └── core/                 # 公共库
│   │       ├── config.rs         # ~/.config/dow/config.toml
│   │       ├── platform.rs       # XDG 路径、平台检测
│   │       ├── github.rs         # Release API、自更新
│   │       ├── archive_db.rs     # SQLite 归档查询
│   │       ├── doc_validator.rs  # 文档格式校验
│   │       ├── doc_root.rs       # .dev-doc 根目录定位
│   │       ├── task_store.rs     # 任务文件读写
│   │       ├── version.rs        # 多分支 VERSION
│   │       ├── claim.rs          # Claim 锁管理
│   │       ├── yaml.rs           # YAML frontmatter 工具
│   │       └── agent_registry.rs # 插件部署
│   ├── dashboard-frontend/       # Web 看板前端（图、看板、查看器）
│   │   ├── graph.js
│   │   ├── views.js
│   │   ├── style.css
│   │   └── vendor/
│   ├── references/               # 注入提示词与文档规范
│   └── Cargo.toml
├── plugin/                       # 共享插件内容（agent 无关）
│   ├── commands/                 # Slash command markdown 文件
│   └── agents/                   # Sub-agent prompt 定义
├── targets/                      # 各 agent 适配层
│   ├── claude/
│   │   ├── plugin.json
│   │   └── hooks.json
│   └── codex/
│       ├── plugin.json
│       └── hooks.json
├── npm/dev-flow/                 # npm 包（@xin_yue/dev-flow）
├── install/                      # 一条命令安装脚本
│   ├── install.sh                # curl | bash
│   └── install.ps1               # irm | iex
├── examples/                     # 快速开始和流程示例
├── devtools/                     # 开发辅助
│   ├── assemble.sh               # 组装 dist/<agent>/
│   └── deploy-local.sh           # 编译 + 本地部署
├── scripts/                      # 工具脚本
├── .github/workflows/
│   ├── release.yml               # CI：tag → 构建 → GitHub Release
│   ├── build-dow.yml             # 构建验证
│   └── test.yml                  # 测试套件
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
