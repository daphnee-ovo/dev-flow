# 头脑风暴记录 — dow 分发与安装机制重构

**日期**：2026-05-30

## 背景与目的

当前 dow 是项目级工具（`scripts/bin/dow`），用户需要 clone 仓库 + 编译 Rust 才能使用。目标是实现一条命令安装，让 dow 成为全局 CLI，支持多 agent 注册和自动更新。

## 关键决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 二进制来源 | GitHub Release 预编译下载 | 用户无需 Rust 工具链 |
| 安装路径 | XDG（~/.local/bin/ + ~/.local/share/ + ~/.config/） | 标准、不污染系统目录 |
| 插件资源获取 | 打包在 Release tarball 中 | 一次下载全部就绪 |
| 项目级 vs 全局 | 全局取代项目级 | hooks 直接调用 PATH 上的 dow |
| 注册机制 | `dow setup --agent claude\|codex\|all` | 参数指定或交互式 TUI |
| 插件安装位置 | 各 agent 自身插件目录（如 ~/.claude/plugins/dev-flow/） | 天然被 agent 识别 |
| 开发目录结构 | 共享核心 + 薄适配层（plugin/ + targets/） | 无重复，单一源 |
| 交叉编译 | 多 runner 矩阵（公开仓库免费） | 原生编译最可靠 |
| 安装脚本 | install.sh（curl \| bash）+ install.ps1（irm \| iex） | 双平台覆盖 |
| 更新检测 | 每日首次后台检查 + stderr 提醒 | 不阻塞不打扰 |
| setup 交互 | TUI 多选（方向键 + 空格 + Enter），含 All 开关 | 直觉操作，预留扩展 |
| 全局指令注入 | setup 时检查 agent 全局指令文件，无 dev-flow 内容则追加 | 引导 agent 全局遵循 |

## 设计方案

### 架构

```
安装流程：
  curl | bash (或 irm | iex)
       │
       ▼
  下载 dow 二进制 → ~/.local/bin/dow
  下载插件 bundle → ~/.local/share/dow/bundle/
       │
       ▼
  自动执行 dow setup（交互式 TUI）
       │
       ├── 复制插件资源 → ~/.claude/plugins/dev-flow/
       ├── 调用 claude plugin marketplace add ... 注册
       ├── 检查 ~/.claude/CLAUDE.md 是否有 dev-flow 引导，无则追加
       └── 记录到 ~/.config/dow/config.toml

运行时：
  Agent hooks → 调用 PATH 上的 dow
  dow 子命令 → 操作当前项目的 .dev-doc/

更新流程：
  dow update
       ├── 检查 GitHub Release 最新版本
       ├── 下载新二进制覆盖 ~/.local/bin/dow
       ├── 下载新 bundle
       └── 更新所有已注册 agent 的插件目录
```

### 组件

**开发环境目录结构：**

```
dev-flow/
├── dow/                      # Rust CLI 源码
│   ├── src/
│   ├── Cargo.toml
│   └── build.sh
├── plugin/                   # 共享插件内容（agent 无关）
│   ├── skills/
│   ├── commands/
│   └── agents/
├── targets/                  # 各 agent 适配层
│   ├── claude/
│   │   ├── plugin.json
│   │   └── hooks.json
│   └── codex/
│       ├── plugin.json
│       └── hooks.json
├── install/                  # 安装脚本
│   ├── install.sh
│   └── install.ps1
├── devtools/                 # 开发辅助
│   ├── assemble.sh           # 组装到 dist/
│   ├── deploy-local.sh       # 组装 + 部署到本地 agent 插件目录
│   └── sync-skill.sh
├── tests/
├── .github/
│   └── workflows/
│       └── release.yml
└── CLAUDE.md
```

**用户侧目录结构：**

```
~/.local/bin/dow                          # 主二进制
~/.local/share/dow/bundle/                # 插件资源（setup 的源）
    ├── claude/
    └── codex/
~/.config/dow/config.toml                 # 配置
~/.claude/plugins/dev-flow/               # Claude Code 插件（由 setup 部署）
```

**dow 新增子命令：**

- `dow setup [--agent claude|codex|all]` — 交互式 TUI 或参数指定注册
- `dow update` — 自更新二进制 + 插件
- `dow self-check` — 显示当前安装状态

**devtools 脚本：**

- `devtools/assemble.sh <claude|codex|all>` — 组装 plugin/ + targets/ → dist/
- `devtools/deploy-local.sh <claude|codex|all>` — assemble + 部署到本地 + 编译 dow

### 数据流

**Release 打包（CI）：**

```
push tag v* → GitHub Actions
  ├── 矩阵编译 5 平台二进制（linux x86/arm, darwin x86/arm, windows x86）
  ├── assemble.sh all → dist/claude/, dist/codex/
  ├── 每平台打包：bin/dow + bundle/ → dow-v{ver}-{platform}.tar.gz
  └── 创建 Release 上传
```

**每日更新检查：**

```
任意 dow 命令执行
  → 读 config.toml 中 last_version_check
  → 超过 24h → spawn 后台进程 GET GitHub API
  → 有新版本 → 写入缓存
  → 下次执行 → stderr 提醒
```

### 错误处理

- 安装时网络失败：明确报错 + 重试建议
- setup 时目标 agent 未安装：跳过并提示
- update 时无网络：跳过检查，不阻塞正常使用
- 插件目录已存在：提示覆盖确认（--force 跳过）

## 约束与边界

- 不做：dow 不负责管理 agent 本身的安装
- 不做：不支持多版本共存
- 不做：不做 agent 插件的运行时代理
- Windows 原生仅 PowerShell，不支持 cmd.exe

## 下一步

建议直接进入 `/spec` — 需求清晰，重点：
1. dow Rust 中 setup/update/self-check 模块设计
2. install.sh / install.ps1 具体实现
3. CI workflow 配置
4. 开发环境迁移计划（当前结构 → 新结构）
