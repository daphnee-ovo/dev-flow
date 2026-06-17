# 使用指南

## 开发环境

- **语言**：Rust（dow CLI）+ Markdown/Shell（插件层）
- **构建**：`bash devtools/deploy-local.sh <claude|codex|all>`
- **测试**：`cd dow && cargo test`
- **dow 二进制位置**：`~/.local/bin/dow`

## 常见任务

### 修改 dow CLI

1. 编辑 `dow/src/` 下的 Rust 源码
2. `cd dow && cargo build` 验证编译
3. `cargo test` 跑测试
4. `bash devtools/deploy-local.sh claude` 部署到本地

### 修改插件命令

1. 编辑 `plugin/commands/<command>.md`
2. `bash devtools/assemble.sh all` 组装到 dist/
3. `bash devtools/deploy-local.sh all` 部署

### 添加新 agent 支持

1. 在 `targets/<agent>/` 创建 `plugin.json` 和 `hooks.json`
2. 在 `devtools/assemble.sh` 中添加对应组装逻辑
3. 确保不破坏已有 agent 的产物

### 测试

- **禁止**在开发环境直接测试（会污染 `.dev-doc/`）
- 使用 `tmp/test_target_project/` 作为测试目标项目
- dow 自身测试：`cd dow && cargo test`
