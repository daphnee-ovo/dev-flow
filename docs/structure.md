# 项目结构

## 目录树

```
dev-flow/
├── dow/                  # Rust CLI 统一调度器（核心二进制）
│   ├── src/              # Rust 源码
│   │   ├── commands/     # 子命令（iterate, rollback, claim, doc, check 等）
│   │   └── core/         # 核心模块（archive_db, doc_validator 等）
│   ├── tests/            # dow 集成测试
│   ├── references/       # 参考资料和模板（编译时嵌入）
│   └── tmp/              # dow 测试临时目录
├── plugin/               # 共享插件内容（跨 agent 通用）
│   ├── commands/         # slash command 定义（.md）
│   ├── agents/           # subagent prompts (audit agents, test agent, task challenger)
│   └── skills/           # skill 定义
├── targets/              # agent 差异化配置
│   ├── claude/           # Claude Code 专用（plugin.json、hooks.json）
│   ├── codex/            # Codex 专用
│   └── kiro/             # Kiro 专用
├── dist/                 # 组装产物（assemble.sh 输出）
│   ├── claude/
│   ├── codex/
│   └── kiro/
├── devtools/             # 开发辅助脚本（不随插件分发）
├── scripts/              # 工具脚本（bin/ 等）
├── npm/                  # npm 包装（平台二进制分发）
├── docs/                 # 持久化项目文档
├── install/              # 安装脚本
├── examples/             # 示例项目
├── tests/                # 项目级测试
├── .dev-doc/             # 流程文档（STATUS、CHANGELOG 等）
│   ├── main/             # 主分支流程
│   └── refactor/         # refactor 分支流程
└── tmp/                  # 测试隔离目录
```

## 模块职责

| 模块 | 职责 |
|------|------|
| `dow/` | 全局 CLI 调度器，处理 hook、状态管理、文档校验、归档、迭代交付、版本回退 |
| `plugin/` | 跨 agent 共享的 command/agent/skill 定义，是插件的逻辑内核 |
| `targets/` | 各 agent 平台的差异化配置（hook 格式、plugin.json 结构） |
| `dist/` | assemble.sh 的输出产物，直接部署到各 agent 插件目录 |
| `devtools/` | 构建、部署、同步等开发期脚本 |
| `install/` | 用户侧安装脚本（dow 二进制 + 插件注册） |
| `tests/` | 项目级集成测试 |
