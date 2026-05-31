# Dev-Flow Plugin

项目全流程管理插件，从需求探索到交付的完整生命周期管理。

## 注意事项
- 禁止直接在开发环境测试导致开发环境的流程管理被污染。如需测试，需要在./tmp/test_target_project 中进行测试


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

`scripts/bin/dow` 是 Rust 编写的统一调度器，所有 hook 和脚本化操作通过它执行。

| 子命令 | 作用 |
|--------|------|
| `dow status` | 读写 STATUS.yaml（`--phase`/`--mode`/`--exec-mode`/`--name`/`--field`） |
| `dow check` | 文档规范检查 |
| `dow issue --list` | 列出未关闭的 issue |
| `dow iterate --topic <t> --type <type> [--files f1 f2...] [-v minor] [--confirm]` | 迭代交付 |
| `dow scan` | 项目扫描 |
| `dow validate` | 校验 .dev-doc 结构 |
| `dow doc <type> [--md\|--json] [-n N] [--source X]` | 生成文档模板 / 查询文档规范 |
| `dow devtest [--task <id>]` | 任务级测试 |
| `dow test [--file <x>]` | 全量测试 |
| `dow archive list [--branch <b>]` | 列出所有归档版本 |
| `dow archive show <version>` | 某版本归档详情 |
| `dow archive tasks [--version v] [--priority P0]` | 查询归档任务 |
| `dow archive issues [--version v] [--severity P0]` | 查询归档 issue |
| `dow archive doc <version> <PRD\|SPEC\|TEST>` | 输出归档文档原文 |
| `dow archive migrate [--delete-originals]` | 从目录迁移到 SQLite |
| `dow archive stats` | 归档统计 |
| `dow hooks context [--codex-hook]` | hook：注入上下文；Codex hook 使用协议 JSON envelope |
| `dow hooks guard <file>` | hook：文件写入守护 |
| `dow hooks post-write <file>` | hook：写后联动 |
| `dow hooks save-changelog` | hook：保存 CHANGELOG |
| `dow version [--set X.Y.Z] [--bump major\|minor\|patch]` | 读写 VERSION |

默认 JSON 输出，`-H` 切换人类友好格式。

构建：`bash dow/build.sh`（本地原生）或 `bash dow/build.sh --dist`（分发模式，输出平台二进制 + wrapper）。

## 文档格式规范（必读）

**创建或写入 .dev-doc 文件时，必须通过 `dow doc <type> --json` 获取格式定义，不要凭记忆或内联模板写入。**

```bash
dow doc task --json    # 获取 task 文件的结构化格式
dow doc issue --json   # 获取 issue 文件的结构化格式
dow doc spec --json    # 获取 SPEC.md 的格式定义
dow doc prd --json     # 获取 PRD.md 的格式定义
dow doc test --json    # 获取 TEST.md 的格式定义
```

`--md` 输出人类可读的完整 markdown 规范，`--json` 输出结构化 JSON（含 template、fields、rules）。
subagent prompt 中应使用 `--json` 输出拼入格式要求。

## Hooks

由 `dow` 统一调度（`hooks/hooks.json`）：

- `UserPromptSubmit`: Codex 使用 `dow hooks context --codex-hook`，Claude 使用 `dow hooks context -H`
- `PreToolUse(Write|Edit|Bash)`: `dow hooks guard`
- `PostToolUse(Write|Edit)`: `dow hooks post-write`
- `Stop`: `dow hooks save-changelog`

## Codex 兼容

- Codex 插件入口：`.codex-plugin/plugin.json`
- Codex skill 入口：`skills/dev-flow/SKILL.md`
- Codex hooks 入口：`hooks.json`（调用 `scripts/bin/dow hooks ...`）
- 命令中要求独立 agent 时，Codex 使用 `spawn_agent`，Claude Code 使用 `Agent`
- `/init` 更新项目级指令时，Codex 优先写 `AGENTS.md`，Claude Code 优先写 `CLAUDE.md`

## 开发辅助工具（devtools/）

`devtools/` 统一存放项目开发过程中使用的辅助脚本和工具，不随插件分发：

| 脚本 | 作用 |
|------|------|
| `sync-skill.sh` | 将 `skills/dev-flow/SKILL.md` 同步到 `.claude/skills/` 和 `.agents/skills/` |
| `sync-plugin.sh` | 同步项目到 Claude Code 插件缓存 |

**修改 SKILL.md 后必须执行 `bash devtools/sync-skill.sh` 同步副本。**

## 目录结构约定

项目使用 `.dev-doc/` 目录管理所有流程文档。
