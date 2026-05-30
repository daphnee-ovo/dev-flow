# Contributing

## 本地开发

```bash
# 克隆仓库
git clone https://github.com/daphnee-ovo/dev-flow.git
cd dev-flow

# 编译 dow + 组装 + 部署到本地（Claude Code 示例）
bash devtools/deploy-local.sh claude

# 验证
dow self-check
```

`deploy-local.sh` 会自动完成：
1. 编译 dow（需要 Rust 工具链）
2. 组装插件（`devtools/assemble.sh`）
3. 安装 dow 到 `~/.local/bin/`
4. 部署插件到对应 agent 目录

### 单独操作

```bash
# 只编译 dow
cd dow && cargo build --release

# 只组装插件
bash devtools/assemble.sh claude    # 或 codex / all

# 查看组装产物
ls dist/claude/
```

## 项目结构

```
dev-flow/
├── dow/                        # Rust CLI 源码
│   ├── src/
│   │   ├── commands/           # 子命令（status, setup, update...）
│   │   ├── hooks/              # Hook 实现
│   │   └── core/               # 公共库（config, platform, github...）
│   └── Cargo.toml
├── plugin/                     # 共享插件内容（agent 无关）
│   ├── skills/                 # Skill 定义
│   ├── commands/               # Slash 命令
│   └── agents/                 # Agent prompt 模板
├── targets/                    # 各 agent 适配层
│   ├── claude/                 # plugin.json + hooks.json
│   └── codex/                  # plugin.json + hooks.json
├── install/                    # 安装脚本
│   ├── install.sh
│   └── install.ps1
├── devtools/                   # 开发辅助
│   ├── assemble.sh             # 组装 plugin/ + targets/ → dist/
│   └── deploy-local.sh         # 编译 + 部署到本地
├── tests/                      # 测试
└── .github/workflows/
    └── release.yml             # tag → 构建 → Release
```

## 开发约定

- 共享内容（skills、commands、agents）放 `plugin/`，一份源码多 agent 共用
- agent 差异（plugin.json、hooks.json）放 `targets/<agent>/`
- hooks 直接调用全局 `dow` 命令，不使用相对路径或 `${CLAUDE_PLUGIN_ROOT}`
- 命令使用运行时中立的写法（子代理 prompt 模板），Claude 用 `Agent`，Codex 用 `spawn_agent`
- 新增命令后需在 `targets/claude/plugin.json` 的 commands 数组中注册
- 修改 plugin/ 或 targets/ 后执行 `bash devtools/deploy-local.sh <agent>` 验证

## 发布流程

1. `dow version --bump minor`（或 major/patch）
2. `git tag v<version> && git push --tags`
3. GitHub Actions 自动构建 5 平台二进制 + 组装 bundle → 发布到 Release
4. 用户执行 `dow update` 即可获取新版本

## 测试

```bash
# Rust 单元测试
cd dow && cargo test

# 集成测试（在隔离目录）
mkdir -p tmp/test_target_project
cd tmp/test_target_project
dow self-check
```
