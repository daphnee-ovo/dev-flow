# Contributing

## 本地开发

```bash
# 克隆仓库
git clone https://github.com/daphnee-ovo/dev-flow.git

# 本地测试（不安装，直接加载）
claude --plugin-dir ./dev-flow

# 或添加为本地 marketplace
/plugin marketplace add ./dev-flow
/plugin install dev-flow@dev-flow
```

### Codex

```bash
# 添加当前仓库为本地 marketplace
codex plugin marketplace add .

# 然后在 Codex 中打开 /plugins，搜索 Dev-Flow 并安装
```

## 项目结构

```
dev-flow/
├── .claude-plugin/
│   ├── plugin.json            # 插件配置（命令注册）
│   └── marketplace.json       # marketplace 元数据
├── .codex-plugin/
│   └── plugin.json            # Codex 插件 manifest
├── .claude/skills/dev-flow/
│   └── SKILL.md               # skill 触发描述
├── skills/dev-flow/
│   └── SKILL.md               # Codex 插件 skill 入口
├── commands/                   # slash 命令定义
│   ├── init.md
│   ├── brainstorm.md
│   ├── prd.md
│   ├── spec.md
│   ├── task.md
│   ├── devtest.md
│   ├── fix.md
│   ├── test.md
│   ├── done.md
│   ├── status.md
│   ├── check.md
│   ├── iterate.md
│   └── mode.md
├── agents/                     # agent prompt 模板
│   ├── prd-agent.md
│   ├── spec-agent.md
│   ├── task-agent.md
│   └── test-agent.md
├── hooks/
│   └── hooks.json              # hook 事件注册
├── hooks.json                  # Codex hook 事件注册
├── scripts/                    # hook 脚本
│   ├── inject-context.sh
│   ├── block-system-tmp.sh
│   ├── check-task-completion.sh
│   ├── check-doc-sync.sh
│   ├── check-phase-completion.sh
│   ├── update-status.sh
│   └── save-session.sh
├── references/                 # 内部参考规范
│   ├── dev-doc-spec.md
│   └── status-template.md
├── CLAUDE.md                   # Claude Code 插件级指令
├── AGENTS.md                   # Codex 插件级指令
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

## 开发约定

- 命令文件使用 YAML frontmatter（description + allowed-tools）
- Claude hook 使用 `${CLAUDE_PLUGIN_ROOT}` 引用插件根目录；Codex 根级 `hooks.json` 使用相对路径调用 `scripts/`
- 命令中涉及独立 agent 时，写成运行时中立的"子代理 prompt 模板"；Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`
- hook 脚本 exit 0 = 通过，exit 2 = 阻断工具执行
- 新增命令后需在 `.claude-plugin/plugin.json` 的 commands 数组中注册；Codex 通过 `commands/` 目录发现命令，仍要确认新命令内容不包含 Claude-only API

## Codex 插件开发注意点

Codex 插件至少需要这些入口：

- `.codex-plugin/plugin.json`：Codex 插件 manifest，声明 `skills`、`hooks`、展示信息和能力
- `skills/<skill-name>/SKILL.md`：Codex skill 入口，负责触发描述和运行约定
- `commands/*.md`：slash command 定义，尽量写成运行时中立的流程说明
- `hooks.json`：Codex 根级 hook 注册，命令路径使用相对路径，如 `./scripts/inject-context.sh`
- `scripts/*.sh`：hook 脚本应可被 Claude 和 Codex 复用，不要只依赖 Claude 专属环境变量

编写 Codex 兼容内容时注意：

- 不要在通用命令里写死 `Agent({...})`、`AskUserQuestion` 等 Claude Code API。写成"启动独立子代理"、"向用户确认"，并注明 Codex 使用 `spawn_agent`。
- 不要在 Codex hook 中使用 `${CLAUDE_PLUGIN_ROOT}`。Codex 插件根目录下的 `hooks.json` 用 `./scripts/...`。
- hook 脚本如果读取工具输入，优先兼容多环境，例如同时支持 `CLAUDE_TOOL_INPUT`、`CODEX_TOOL_INPUT` 和 stdin。
- 项目级指令文件不要只更新 `CLAUDE.md`。Codex 项目优先更新 `AGENTS.md`；如果两个文件都存在，保持 dev-flow 段落一致。
- `.codex-plugin/plugin.json` 不使用 Claude 的 `commands` 字段。Codex 的命令文件可以放在根级 `commands/`。
- 修改 manifest 后必须跑 JSON 校验：`python3 -m json.tool .codex-plugin/plugin.json`。
- 修改 hook 后必须跑 JSON 校验：`python3 -m json.tool hooks.json`。

## 先开发 Claude Code 再同步到 Codex

如果先按 Claude Code 方式开发，合并前必须逐项同步：

- `.claude-plugin/plugin.json` 新增或删除命令后，确认根级 `commands/` 内容也适合 Codex 读取。
- `.claude/skills/dev-flow/SKILL.md` 有行为变更时，同步到 `skills/dev-flow/SKILL.md`。
- `hooks/hooks.json` 有 hook 变更时，同步到根级 `hooks.json`，并把 `${CLAUDE_PLUGIN_ROOT}/scripts/...` 改成 `./scripts/...`。
- `scripts/*.sh` 不要只读取 Claude 环境变量；新增输入读取逻辑时同步检查 Codex。
- 命令文档里如果新增了 `Agent({...})` 示例，改成运行时中立的子代理 prompt 模板。
- `/init`、`/mode` 等会写项目指令的命令，确认同时覆盖 `AGENTS.md` 和 `CLAUDE.md` 的规则。
- README 安装说明如果变更，Claude Code 和 Codex CLI 两段都要更新。
- 最后用 Codex 临时环境验证 marketplace 能加载：

```bash
python3 -m json.tool .codex-plugin/plugin.json >/dev/null
python3 -m json.tool hooks.json >/dev/null
codex plugin marketplace add .
codex debug prompt-input "/status"
```

`codex debug prompt-input "/status"` 的输出中应能看到 `dev-flow:dev-flow` skill。

## 测试

Claude Code 修改后本地验证：

```bash
# 加载插件
claude --plugin-dir ./dev-flow

# 验证命令注册
/plugin

# 测试具体命令
/init
/status
```

Codex 修改后本地验证：

```bash
codex plugin marketplace add .
codex debug prompt-input "/status"
```

检查输出中应出现 `dev-flow:dev-flow` skill，且 `.codex-plugin/plugin.json`、`hooks.json` 都是合法 JSON。
