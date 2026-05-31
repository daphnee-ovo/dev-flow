# Dev-Flow Codex UserPromptSubmit Hook Issue

## 背景

当前问题发生在 dev-flow 插件的 Codex hook 上。用户在 `/home/xinyue/session` 中初始化并启用 dev-flow 后，Codex 触发 `UserPromptSubmit` hook，但界面报错：

```text
UserPromptSubmit hook (failed)
error: hook returned invalid user prompt submit JSON output
```

用户已经更新过 `dow`，并确认：

```bash
dow hooks context
```

可以输出合法 JSON，例如：

```json
{
  "branch": "master",
  "version": "0.1.0",
  "version_tag": "no-tag",
  "mode": "fast",
  "phase": "PRD",
  "exec_mode": "step",
  "doc_root": ".dev-doc/master",
  "tasks": {
    "total": 0,
    "done": 0,
    "by_priority": {}
  },
  "issues": 0
}
```

但这仍然会报错，说明问题不是“不是 JSON”，而是“不是 Codex `UserPromptSubmit` hook 期望的 JSON 协议结构”。


## 根因判断

`dow hooks context` 当前输出的是 dev-flow 自己的业务上下文对象：

```json
{
  "branch": "...",
  "mode": "...",
  "phase": "..."
}
```

这个 JSON 对人和 `dow` 调试有用，但不是 Codex `UserPromptSubmit` hook runner 接受的专用输出结构。

也就是说：

- `dow hooks context -H`：人类可读文本，肯定不适合新版 Codex JSON hook 输出。
- `dow hooks context`：合法 JSON，但仍然不是 `UserPromptSubmit` 的 hook response schema。
- 所以需要新增一个显式模式，让 Codex hook 使用 Codex 专用 envelope。

用户明确建议正确做法是增加：

```bash
dow hooks context --codex-hook
```

这个方向是合理的，优于新增子命令，因为它保留 `context` 的语义，同时让 hook 协议输出变成显式 opt-in。

## 建议改法

### 1. 修改 CLI 参数

文件：

```text
/home/xinyue/mythology/dev-flow/dow/src/cli.rs
```

当前：

```rust
pub enum Commands {
    /// Hook 子命令
    Hooks {
        #[command(subcommand)]
        command: HooksCommands,
    },
}

#[derive(Subcommand)]
pub enum HooksCommands {
    /// 注入上下文
    Context,
    ...
}
```

建议改成：

```rust
pub enum Commands {
    /// Hook 子命令
    Hooks {
        /// 输出 Codex hook 协议 JSON
        #[arg(long)]
        codex_hook: bool,

        #[command(subcommand)]
        command: HooksCommands,
    },
}

#[derive(Subcommand)]
pub enum HooksCommands {
    /// 注入上下文
    Context,
    ...
}
```

### 2. 修改 main 分发

文件：

```text
/home/xinyue/mythology/dev-flow/dow/src/main.rs
```

当前：

```rust
Commands::Hooks { command } => match command {
    HooksCommands::Context => hooks::context::run(human),
    ...
},
```

建议：

```rust
Commands::Hooks { codex_hook, command } => match command {
    HooksCommands::Context => hooks::context::run(human, codex_hook),
    ...
},
```

### 3. 修改 context 输出

文件：

```text
/home/xinyue/mythology/dev-flow/dow/src/hooks/context.rs
```

目标行为：

- `dow hooks context`：保持现有普通 JSON 输出，避免破坏调试和脚本。
- `dow hooks context -H`：保持现有人类可读输出。
- `dow hooks context --codex-hook`：输出 Codex hook 协议 JSON。

建议新增结构：

```rust
#[derive(Serialize)]
struct CodexHookSpecificOutput {
    #[serde(rename = "hookEventName")]
    hook_event_name: String,
    #[serde(rename = "additionalContext")]
    additional_context: String,
}

#[derive(Serialize)]
struct CodexUserPromptSubmitOutput {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: CodexHookSpecificOutput,
}
```

然后把当前 `print_human(&output_data)` 使用的文本格式提取成：

```rust
fn format_human_context(data: &ContextOutput) -> String
```

让：

```rust
print_human(&output_data)
```

和：

```rust
additional_context: format_human_context(&output_data)
```

复用同一份上下文文本。

预期 `--codex-hook` 输出大致为：

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "{\n  \"branch\": \"master\",\n  \"version\": \"0.1.0\",\n  \"version_tag\": \"no-tag\",\n  \"mode\": \"fast\",\n  \"phase\": \"PRD\",\n  \"exec_mode\": \"step\",\n  \"doc_root\": \".dev-doc/master\",\n  \"tasks\": {\n    \"total\": 0,\n    \"done\": 0,\n    \"by_priority\": {}\n  },\n  \"issues\": 0\n}"
  }
}
```

用户审批意见：`additionalContext` 使用 JSON 字符串格式，即与原来的 `dow hooks context` 输出保持一致。

注意：字段名 `additionalContext` 是基于当前判断的建议，需要在实现时用 Codex 当前 hook schema 最终确认。如果 Codex schema 使用不同字段，应以 Codex 实际 schema 为准。

### 4. 修改 Codex hook 配置模板

文件：

```text
/home/xinyue/mythology/dev-flow/targets/codex/hooks.json
```

把 Codex 的 `UserPromptSubmit` 从：

```json
"command": "dow hooks context -H"
```

改成：

```json
"command": "dow hooks context --codex-hook"
```

不要同步改 Claude 的 `targets/claude/hooks.json`，除非确认 Claude Code 也接受同一协议。当前问题只发生在 Codex。

### 5. 同步当前已安装配置

修完源码并构建/安装后，确保以下位置最终也变成：

```json
"command": "dow hooks context --codex-hook"
```

位置：

```text
/home/xinyue/.codex/plugins/plugins/dev-flow/hooks.json
/home/xinyue/.codex/plugins/plugins/dev-flow/hooks/hooks.json
/home/xinyue/.codex/plugins/cache/dev-flow-local/dev-flow/3.8.10/hooks.json
/home/xinyue/.codex/plugins/cache/dev-flow-local/dev-flow/3.8.10/hooks/hooks.json
```

之前只改了其中两份，后面发现还有：

```text
/home/xinyue/.codex/plugins/plugins/dev-flow/hooks/hooks.json
```

仍然保留 `dow hooks context -H`。需要一起处理，避免 Codex 从另一个入口读到旧配置。

## 验证步骤

在 `/home/xinyue/mythology/dev-flow` 修改并构建后：

```bash
dow hooks context --codex-hook
```

应输出合法 JSON，且顶层应是 hook response envelope，而不是裸 `branch/mode/phase`。

再检查安装配置：

```bash
rg -n "dow hooks context" /home/xinyue/.codex/plugins/plugins/dev-flow /home/xinyue/.codex/plugins/cache/dev-flow-local/dev-flow/3.8.10
```

所有 Codex `UserPromptSubmit` 都应指向：

```text
dow hooks context --codex-hook
```

最后重载 Codex，再触发一次用户消息。预期不再出现：

```text
invalid user prompt submit JSON output
```

## 不要重复走的路

- 不要只把 `dow hooks context -H` 改成 `dow hooks context`。这已经试过，仍然报错。
- 不要把普通 `dow hooks context` 的默认输出直接改成 hook envelope，会破坏 CLI 调试和可能已有脚本。
- 不要只改 active plugin 目录，cache 和 `hooks/hooks.json` 也可能被读取。
- 不要把 Claude hook 一起改掉，除非确认 Claude Code 的 `UserPromptSubmit` schema 与 Codex 一致。
