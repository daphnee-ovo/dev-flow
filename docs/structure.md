# 项目结构

## 目录树

```
dev-flow/
├── dow/                  # Rust CLI 统一调度器（核心二进制）
│   ├── src/              # Rust 源码
│   │   ├── commands/     # 子命令（iterate, rollback, claim, dashboard 等）
│   │   ├── core/         # 核心模块（archive_db, claim, doc_validator 等）
│   │   └── dashboard/    # dashboard web 服务（axum + SSE + data）
│   ├── dashboard-frontend/ # 前端静态资源（D3/Dagre/Marked，编译时嵌入）
│   ├── tests/            # dow 集成测试
│   ├── references/       # Reference texts (prompts, schemas, help)
│   │   └── binary/       # Subset compiled into dow via include_str!
│   ├── docs/             # Developer-facing specs, not compiled into dow
│   └── tmp/              # dow 测试临时目录
├── vscode-extension/     # VS Code 扩展（嵌入 dashboard webview）
├── plugin/               # 共享插件内容（跨 agent 通用）
│   ├── commands/         # slash command 定义（.md）
│   └── agents/           # subagent prompts (audit agents, test agent, task challenger)
├── targets/              # agent 差异化配置
│   ├── claude/           # Claude Code 专用（plugin.json、hooks.json）
│   ├── codex/            # Codex 专用
│   ├── kiro/             # Kiro 专用
│   └── pi/               # Pi 专用
├── dist/                 # 组装产物（assemble.sh 输出）
│   ├── claude/
│   ├── codex/
│   ├── kiro/
│   └── pi/
├── devtools/             # 开发辅助脚本（不随插件分发）
├── npm/                  # npm 包装（平台二进制分发）
├── docs/                 # 持久化项目文档
├── install/              # 安装脚本
├── tests/                # 项目级测试
├── .dev-doc/             # 流程文档（STATUS、CHANGELOG 等）
│   ├── main/             # 主分支流程
│   └── refactor/         # refactor 分支流程
└── tmp/                  # 测试隔离目录
```

## 模块职责

| 模块 | 职责 |
|------|------|
| `dow/` | 全局 CLI 调度器，处理 hook、状态管理、文档校验、归档、迭代交付、dashboard、版本回退 |
| `dow/dashboard-frontend/` | dashboard 前端（D3 依赖图、看板、文档预览），编译时嵌入二进制 |
| `vscode-extension/` | VS Code 扩展，status bar 触发 + webview 嵌入 dashboard |
| `plugin/` | 跨 agent 共享的 command/agent 定义，是插件的逻辑内核（skills 由 assemble 生成到 dist） |
| `targets/` | 各 agent 平台的差异化配置（hook 格式、plugin.json 结构） |
| `dist/` | assemble.sh 的输出产物，直接部署到各 agent 插件目录 |
| `devtools/` | 构建、部署、同步等开发期脚本 |
| `install/` | 用户侧安装脚本（dow 二进制 + 插件注册） |
| `tests/` | 项目级集成测试 |
