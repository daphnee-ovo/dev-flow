# SPEC: dow 分发与安装机制重构

## Goal

将 dow 从项目级工具升级为全局 CLI，实现一条命令安装（`curl | bash`）、多 agent 注册（`dow setup`）、自更新（`dow update`），同时重构开发目录结构为共享核心 + 适配层架构。

## Design

### 模块划分

在 dow Rust 源码中新增以下模块：

```
dow/src/
├── commands/
│   ├── setup.rs        # dow setup 子命令（TUI 交互 + agent 注册）
│   ├── update.rs       # dow update 子命令（自更新二进制 + 插件）
│   └── self_check.rs   # dow self-check 子命令（安装状态诊断）
├── core/
│   ├── config.rs       # ~/.config/dow/config.toml 读写
│   ├── platform.rs     # 平台检测、路径约定（XDG）
│   ├── github.rs       # GitHub Release API 交互（版本检查、下载）
│   └── agent_registry.rs  # agent 插件目录发现与文件部署
```

现有模块不变，仅在 cli.rs 增加 Setup/Update/SelfCheck 枚举变体，main.rs 增加路由。

### 安装路径约定

| 路径 | 用途 |
|------|------|
| `~/.local/bin/dow` | 主二进制 |
| `~/.local/share/dow/bundle/<agent>/` | 插件资源包（setup 源） |
| `~/.config/dow/config.toml` | 全局配置（已注册 agent、last_version_check） |
| `~/<agent-plugin-dir>/dev-flow/` | 各 agent 的插件安装位置 |

### 开发目录重构

```
dev-flow/
├── dow/                  # Rust CLI（不变）
├── plugin/               # 共享插件内容（agent 无关）
│   ├── skills/
│   ├── commands/
│   └── agents/
├── targets/              # 各 agent 适配层
│   ├── claude/           # plugin.json + hooks.json
│   └── codex/            # plugin.json + hooks.json
├── install/              # install.sh + install.ps1
├── devtools/             # assemble.sh, deploy-local.sh, sync-skill.sh
└── .github/workflows/release.yml
```

当前 `scripts/bin/`、`.claude-plugin/`、`.codex-plugin/` 迁移到新结构后移除。`skills/`、`commands/`、`agents/` 移入 `plugin/`。`hooks/` 拆分为 `targets/<agent>/hooks.json`。

### 新增依赖

| crate | 用途 | 理由 |
|-------|------|------|
| `reqwest` (blocking) | HTTP 下载 + GitHub API | 稳定、支持 TLS |
| `toml` | config.toml 读写 | 标准选择 |
| `dialoguer` | TUI 多选交互 | 轻量、跨平台终端 UI |
| `flate2` + `tar` | 解压 tarball | Release 包解压 |

### 关键流程

**install.sh**：检测平台 -> 下载 tarball -> 解压 dow 到 `~/.local/bin/` -> 解压 bundle 到 `~/.local/share/dow/` -> 自动执行 `dow setup`。

**dow setup**：读取 bundle -> TUI 选择 agent -> 复制插件到 agent 插件目录 -> 检查全局指令文件并追加引导 -> 写入 config.toml。

**dow update**：GET GitHub API latest release -> 比较版本 -> 下载新 tarball -> 替换二进制 + bundle -> 重新部署已注册 agent。

**每日版本检查**：任意命令执行 -> 读 config.toml last_version_check -> 超 24h -> spawn 后台检查 -> 有新版本写缓存 -> 下次 stderr 提醒。

### CI Release 流程

```yaml
on: push tags v*
jobs:
  build:
    strategy:
      matrix:
        - linux-x86_64, linux-aarch64, darwin-arm64, darwin-x86_64, windows-x86_64
    steps:
      - cargo build --release --target <triple>
      - devtools/assemble.sh all
      - tar: bin/dow + bundle/ -> dow-v{ver}-{platform}.tar.gz
      - gh release upload
```

## Risks

| 风险 | 影响 | 缓解 |
|------|------|------|
| reqwest 增加编译体积和编译时间 | 二进制从 ~5MB 增长到 ~10MB | 可接受；用 blocking 特性避免 tokio 全量引入 |
| GitHub API rate limit | 未认证 60次/h | update 场景足够；可后续加 token |
| install.sh 在非标准环境失败 | 部分用户无法安装 | 提供手动安装文档作为 fallback |
| 目录迁移破坏现有开发流程 | 开发者短期混乱 | 分阶段迁移：先建新结构，再移文件，最后删旧路径 |

## Acceptance

- SPEC-AC-001: `curl -fsSL <url>/install.sh | bash` 在 Linux x86_64 和 macOS arm64 上成功安装 dow 到 `~/.local/bin/dow` 并可执行 `dow --version`
- SPEC-AC-002: `dow setup --agent claude` 将插件部署到 `~/.claude/plugins/dev-flow/` 且目录结构完整（含 skills/、hooks.json）
- SPEC-AC-003: `dow update` 能检测到新版本、下载并替换本地二进制，替换后 `dow --version` 显示新版本号
- SPEC-AC-004: `dow self-check` 输出当前安装状态（版本、已注册 agent、路径完整性）
- SPEC-AC-005: CI 在 push tag 时自动构建 5 平台二进制并上传 GitHub Release
- SPEC-AC-006: 开发目录重构后 `devtools/assemble.sh claude` 产出 `dist/claude/` 包含完整可部署插件
- SPEC-AC-007: 每日首次执行 dow 命令时，若有新版本可用则 stderr 输出一行提醒，不阻塞命令执行

## Test Plan

- 单元测试：`core/config.rs`（config 读写）、`core/platform.rs`（路径生成）、`core/github.rs`（版本比较逻辑，mock HTTP）
- 集成测试：在 `tmp/test_target_project/` 中模拟 `dow setup` + `dow self-check` 流程，验证文件部署正确
- CI 测试：release workflow dry-run 验证矩阵编译和 assemble 产出
- 手动验证：install.sh 在干净 Linux 容器中端到端执行

## Self Check
- [x] 目标清楚
- [x] 验收可测
- [x] 与当前 mode 匹配
