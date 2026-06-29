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

### 版本回退（rollback）

如果 iterate 后发现问题需要回退：

```bash
dow rollback --list           # 列出可回退的版本
dow rollback --version 0.1.1  # 回退指定版本
```

rollback 会：
- 从 archive.db 还原 task/issue/doc 文件（保持 done_/closed_ 前缀）
- 处理文件 seq 冲突（现有文件顺延）
- 标记该迭代为 rolled back
- 重置阶段为 DEV

注意：rollback 不撤销 git commit，仅还原流程状态。

### 启动 Dashboard

```bash
dow dashboard              # 浏览器自动打开
dow dashboard --no-open    # 不打开浏览器（VS Code 扩展使用）
dow dashboard --port 9801  # 指定端口
```

VS Code 中安装 `vscode-extension/` 扩展后，status bar 按钮直接在编辑器内打开 dashboard。

### 修改 VS Code 扩展

1. 编辑 `vscode-extension/src/extension.ts`
2. `cd vscode-extension && npm run compile`
3. `npx vsce package --allow-missing-repository`
4. `code --install-extension dow-dashboard-0.1.0.vsix --force`

### 测试

- **禁止**在开发环境直接测试（会污染 `.dev-doc/`）
- 使用 `tmp/test_target_project/` 作为测试目标项目
- dow 自身测试：`cd dow && cargo test`
