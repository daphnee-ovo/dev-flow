# Dev-Flow Plugin

项目全流程管理插件，从需求探索到交付的完整生命周期管理。

## 注意事项
- 禁止直接在开发环境测试导致开发环境的流程管理被污染。如需测试，需要在./tmp/test_target_project 中进行测试
- 禁止在 `dow/` 目录下运行 dow 命令或创建 `.dev-doc/`：dow 会在当前工作目录就地初始化流程文档，在 `dow/` 内运行会生成 `dow/.dev-doc/` 污染源码树。本项目的流程文档只应位于仓库根的 `.dev-doc/`。运行 dow 测试请用 `cd dow && cargo test`（测试在 tmpdir 隔离），不要手动 `dow init`。

## 命令

| 命令 | 作用 |
|------|------|
| `/init` | 初始化 dev-flow 项目（创建 .dev-doc、选择模式） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动 PRD agent，进入需求探索阶段 |
| `/spec` | 启动 SPEC agent，进入技术规范阶段 |
| `/task` | 启动 TASK agent，进入任务拆解阶段 |
| `/issue` | 手动创建 issue 文件 |
| `/devtest` | 开发中例行测试（任务级验证） |
| `/fix` | 自动读取未关闭 issue 并修复 |
| `/test` | 启动完整 TEST agent（项目级全量验证） |
| `/status` | 报告当前项目状态和进度 |
| `/check` | 检查开发工作是否已同步到 .dev-doc |
| `/iterate` | 迭代交付（检查 + 归档 + commit & tag + bump） |
| `/mode` | 选择开发模式（full/quick/fast/mvp；audit 为自动触发） |

## 流程

```
头脑风暴(BRAINSTORM) → 需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 迭代(ITERATE) → 下一轮
        可选                                                  │                ↑
                                                              │  例行TEST      │
                                                              │  (任务级循环)    │ 项目TEST
                                                              └───────────────→│
```

## dow CLI

`dow` 是 Rust 编写的全局 CLI 统一调度器，所有 hook 和脚本化操作通过它执行。安装后位于 `~/.local/bin/dow`。

| 子命令 | 作用 |
|--------|------|
| `dow task create [flags \| stdin JSON]` | 创建任务 |
| `dow task list [--all]` | 列出待处理任务（默认 pending） |
| `dow task show <ID>` | 任务详情 |
| `dow task done <ID>` | 标记任务完成 |
| `dow task reopen <ID> [--confirm TRO-xxx]` | 重开已完成任务 |
| `dow task schema` | 输出任务字段定义 |
| `dow issue create [flags \| stdin JSON]` | 创建 issue |
| `dow issue list [--all]` | 列出 open issue（默认 open） |
| `dow issue show <ID>` | issue 详情 |
| `dow issue close <ID>` | 关闭 issue |
| `dow issue reopen <ID> [--confirm IRO-xxx]` | 重开已关闭 issue |
| `dow issue schema` | 输出 issue 字段定义 |
| `dow changelog list` | 列出当前 CHANGELOG 条目 |
| `dow changelog add --text "..."` | 追加 CHANGELOG 条目 |
| `dow prd create` | 创建 PRD.md |
| `dow prd schema` | 输出 PRD 格式定义 |
| `dow spec create` | 创建 SPEC.md |
| `dow spec schema` | 输出 SPEC 格式定义 |
| `dow brainstorm create` | 创建 BRAINSTORM.md |
| `dow brainstorm schema` | 输出 BRAINSTORM 格式定义 |
| `dow status` | 读取 STATUS.yaml |
| `dow status set --phase/--mode/--exec-mode/--name/--goals-minor/--goals-major` | 写入 STATUS.yaml |
| `dow init --name <n> --mode <m>` | 初始化 dev-flow 工作流管理 |
| `dow lint [--fix]` | 检查 .dev-doc 结构 + 规范 + 一致性（合并原 check/validate/fix） |
| `dow test [--task <ID>] [--file <x>]` | 运行测试（全量 / 任务级） |
| `dow iterate --topic <t> --type <type> [--files f1 f2...] [-v patch] [--confirm ITR-xxx]` | 迭代交付 |
| `dow rollback --version <v>` | 版本回退（仅回退流程状态，不撤销 git commit） |
| `dow rollback --list` | 列出可回退的版本 |
| `dow claim [IDs...] [--revoke]` | 声明/释放当前工作关联的 task/issue |
| `dow scan` | 项目扫描 |
| `dow version [--set X.Y.Z] [--bump major\|minor\|patch]` | 读写 VERSION（禁止直接编辑文件） |
| `dow archive list [--branch <b>]` | 列出所有归档版本 |
| `dow archive show <version>` | 某版本归档详情 |
| `dow archive tasks [--version v] [--priority P0]` | 查询归档任务 |
| `dow archive issues [--version v] [--severity P0]` | 查询归档 issue |
| `dow archive doc <version> <PRD\|SPEC\|TEST>` | 输出归档文档原文 |
| `dow archive migrate [--delete-originals]` | 从目录迁移到 SQLite |
| `dow archive stats` | 归档统计 |
| `dow hooks context [--codex-hook]` | hook：注入上下文 |
| `dow hooks guard <file>` | hook：文件写入守护 |
| `dow hooks post-write <file>` | hook：写后联动 |
| `dow hooks post-bash [command]` | hook：Bash 执行后检测分支切换 |
| `dow hooks save-changelog [--codex-hook]` | hook：保存 CHANGELOG |
| `dow setup [--agent claude\|codex\|all]` | 注册插件到 agent（交互式 TUI） |
| `dow update` | 自更新二进制 + 插件 |
| `dow self-check` | 查看安装状态和健康度 |

默认 JSON 输出，`-H` 切换人类友好格式。

构建与部署：`bash devtools/deploy-local.sh <claude|codex|all>`（编译 + 组装 + 本地部署）。

## 文档格式规范（必读）

**创建 .dev-doc 文件时，必须通过 `dow <resource> create` 命令，不要直接创建文件。**
**获取格式定义时，通过 `dow <resource> schema` 命令。**

```bash
dow task schema        # 获取 task 字段定义
dow issue schema       # 获取 issue 字段定义
dow spec schema        # 获取 SPEC.md 格式定义
dow prd schema         # 获取 PRD.md 格式定义
dow brainstorm schema  # 获取 BRAINSTORM.md 格式定义
dow changelog schema   # 获取 CHANGELOG 格式定义
```

结构型文件（task/issue/STATUS/CHANGELOG）的全部操作必须通过 dow 命令，不允许 agent 直接 Read/Write。
文档型文件（PRD.md/SPEC.md/BRAINSTORM.md）创建由 dow 管理，后续编辑 agent 可直接操作。

## Hooks

由 `dow` 统一调度（`targets/<agent>/hooks.json`）：

- `UserPromptSubmit`: Codex 使用 `dow hooks context --codex-hook`，Claude 使用 `dow hooks context -H`
- `PreToolUse(Write|Edit|Bash)`: `dow hooks guard`
- `PostToolUse(Write|Edit)`: `dow hooks post-write`
- `PostToolUse(Bash)`: `dow hooks post-bash`（检测分支切换）
- `Stop`: Codex 使用 `dow hooks save-changelog --codex-hook`，Claude 使用 `dow hooks save-changelog`

## 多 Agent 支持

- 严禁更新 A agent 相关内容的时候导致 B agent的支持被破坏
- 共享内容（skills、commands、agents）放 `plugin/`
- agent 差异（plugin.json、hooks.json）放 `targets/<agent>/`
- Codex 不支持直接注册 slash command；`assemble.sh codex` 会把 `plugin/commands/<command>.md` 转换为 `skills/<command>/SKILL.md`，并使用 skill 语义的触发描述
- Kiro 同样不支持 slash command；`assemble.sh kiro` 会转换为 `skills/dev-flow-<command>/SKILL.md`，agents 转为 JSON 格式
- Kiro hooks 是每个 hook 独立 JSON 文件放 `.kiro/hooks/`，格式为 `{name, when: {type, toolTypes?}, then: {type: "runCommand", command}}`
- 命令中要求独立 agent 时，Codex 使用 `spawn_agent`，Claude Code 使用 `Agent`，Kiro 使用 subagent
- `/init` 更新项目级指令时，Codex 优先写 `AGENTS.md`，Claude Code 优先写 `CLAUDE.md`，Kiro 优先写 `.kiro/steering/`

## 开发辅助工具（devtools/）

`devtools/` 统一存放项目开发过程中使用的辅助脚本和工具，不随插件分发。

| 脚本 | 作用 |
|------|------|
| `assemble.sh` | 组装 plugin/ + targets/ → dist/<agent>/ |
| `deploy-local.sh` | 编译 + 组装 + 部署到本地 agent 插件目录 |
| `sync-skill.sh` | 将 SKILL.md 同步到各副本位置 |

## 目录结构约定

项目使用 `.dev-doc/` 目录管理所有流程文档。
