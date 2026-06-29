# Changelog

- 2026-06-28 feat: 新增 dow dashboard 命令 — 本地 web 可视化面板，展示项目状态/依赖图/文档/看板（axum + SSE + Dagre + D3）
## 2026-06-26
- 12:52 feat: Add dow task update and dow issue update subcommands
- 15:49 docs: add IMPORTANT rule — no code changes without open task/issue

## 2026-06-28
- 17:50 feat: 添加依赖 + dashboard 子命令骨架 + 模块结构
- 17:58 feat: 实现 data.rs — 读取 .dev-doc/ 序列化为统一 JSON
- 18:06 feat: 实现 server.rs + watcher.rs — axum SSE + notify + 自动退出

## 2026-06-29
- 11:10 feat: add agent_id (TTY) to claim and advisory guard warning
- 15:00 docs: mark remaining tasks done
- 17:32 fix: detect_agent_id cross-platform + guard ask for AI config paths
