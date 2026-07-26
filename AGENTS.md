# Dev-Flow Plugin

项目全流程管理插件，从需求探索到交付的完整生命周期管理。

## Language Policy

- Use English consistently for default workflow prompts, skill metadata,
  generated artifacts, build output, and new user-facing messages.
- Keep non-English text only in explicitly localized files, such as
  `README.zh-CN.md`, or when preserving user-provided content.

## 注意事项
- 禁止直接在开发环境测试导致开发环境的流程管理被污染。如需测试，需要在./tmp/test_target_project 中进行测试
- 严禁更新 A agent 相关内容的时候导致 B agent的支持被破坏


## 命令

| 命令 | 作用 |
|------|------|
| `/init` | 初始化 dev-flow 项目（创建 .dev-doc、选择模式） |
| `/brainstorm` | 实现前的协作式需求探索与设计 |
| `/prd` | 启动 PRD agent，进入需求探索阶段 |
| `/spec` | 启动 SPEC agent，进入技术规范阶段 |
| `/task` | 启动 TASK agent，进入任务拆解阶段 |
| `/issue` | 手动创建 issue 文件 |
| `/fix` | User-triggered workflow: read, claim, fix, verify, and close open issues |
| `/test` | 执行 `dow test` 全量测试 |
| `/status` | 报告当前项目状态和进度 |
| `/check` | 执行 `dow doctor` 检查文档和项目状态 |
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

`PATH 中的 `dow`（默认安装路径为 `~/.local/bin/dow`）` 是 Rust 编写的统一调度器，所有 hook 和脚本化操作通过它执行。

| 子命令 | 作用 |
|--------|------|
| `dow status` | 读取 STATUS.yaml |
| `dow status set --phase/--mode/--exec-mode/--name/--goals-minor/--goals-major` | 更新 STATUS.yaml 字段 |
| `dow doctor [--fix]` | 统一诊断、校验并可修复 .dev-doc；`dow fix` 是兼容别名 |
| `dow issue list [--all]` | 列出 issue |
| `dow iterate --topic <t> --type <type> [--files f1 f2...] [-v minor] [--confirm ITR-xxxxxx]` | 迭代交付 |
| `dow scan` | 项目扫描 |
| `dow task/issue/prd/spec/brainstorm/changelog schema` | 获取对应资源的格式定义 |
| `dow test` | 全量测试 |
| `dow test <TASK-ID>` | 执行指定 Task 关联的测试 |
| `dow archive list [--branch <b>]` | 列出所有归档版本 |
| `dow archive show <version>` | 某版本归档详情 |
| `dow archive tasks [--version v] [--priority P0]` | 查询归档任务 |
| `dow archive issues [--version v] [--severity P0]` | 查询归档 issue |
| `dow archive doc <version> <PRD\|SPEC\|TEST\|BRAINSTORM>` | 输出归档文档原文 |
| `dow archive migrate [--delete-originals]` | 从目录迁移到 SQLite |
| `dow archive stats` | 归档统计 |
| `dow hooks context [--codex-hook]` | hook：注入上下文；Codex hook 使用协议 JSON envelope |
| `dow hooks guard <file>` | hook：文件写入守护 |
| `dow hooks post-write <file>` | hook：写后联动 |
| `dow hooks save-changelog [--codex-hook]` | hook：保存 CHANGELOG；Codex hook 使用 Stop 协议 JSON |
| `dow version [--set X.Y.Z] [--bump major\|minor\|patch]` | 读写 VERSION |

默认 JSON 输出，`-H` 切换人类友好格式。

构建：`cargo build --manifest-path dow/Cargo.toml`；本地 agent 部署使用 `bash devtools/deploy-local.sh <claude|codex|kiro|all>`。

## 文档格式规范（必读）

**创建或写入 .dev-doc 文件时，先通过对应的 `schema` 子命令获取格式定义，不要凭记忆或内联模板写入。**

```bash
dow task schema       # 获取 task 文件的结构化格式
dow issue schema      # 获取 issue 文件的结构化格式
dow spec schema       # 获取 SPEC.md 的格式定义
dow prd schema        # 获取 PRD.md 的格式定义
dow brainstorm schema # 获取 BRAINSTORM.md 的格式定义
dow changelog schema  # 获取 CHANGELOG 的格式定义
```

subagent prompt 中应使用对应的 schema 输出拼入格式要求。

## Hooks

由 `dow` 统一调度（`targets/<agent>/hooks.json`）：

- `UserPromptSubmit`: Codex 使用 `dow hooks context --codex-hook`，Claude 使用 `dow hooks context -H`
- `PreToolUse(Write|Edit|Bash)`: `dow hooks guard`
- `PostToolUse(Write|Edit)`: `dow hooks post-write`
- `Stop`: Codex 使用 `dow hooks save-changelog --codex-hook`，Claude 使用 `dow hooks save-changelog`

## Codex 兼容

- Codex 插件入口：`.codex-plugin/plugin.json`
- Codex skill 入口：组装后每个命令位于 `skills/<command>/SKILL.md`
- Codex 不支持直接注册 slash command；组装时会把 `plugin/commands/<command>.md` 转换为 `skills/<command>/SKILL.md`，并使用 skill 语义的触发描述
- Codex hooks 入口：`hooks.json`（调用 `PATH 中的 `dow`（默认安装路径为 `~/.local/bin/dow`） hooks ...`）
- 命令中要求独立 agent 时，Codex 使用 `spawn_agent`，Claude Code 使用 `Agent`
- `/init` 更新项目级指令时，Codex 优先写 `AGENTS.md`，Claude Code 优先写 `CLAUDE.md`

## 开发辅助工具（devtools/）

`devtools/` 统一存放项目开发过程中使用的辅助脚本和工具，不随插件分发：

| 脚本 | 作用 |
|------|------|
| `assemble.sh` | 将共享命令和 agent 组装为 Claude、Codex、Kiro 产物 |
| `deploy-local.sh` | 编译并部署本地 agent 插件 |

**修改 `plugin/commands/` 或 `plugin/agents/` 后必须执行 `bash devtools/assemble.sh all` 验证生成产物。**

## 目录结构约定

项目使用 `.dev-doc/` 目录管理所有流程文档。
