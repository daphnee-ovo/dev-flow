# 设计决策记录

## dow 作为统一调度器

- **日期**：2025-05
- **决策**：所有 hook 和流程操作通过 Rust 编写的 `dow` CLI 统一调度
- **理由**：Shell 脚本难以跨平台、难以维护；Rust 提供类型安全和高性能；单一入口简化 hook 配置
- **后果**：新功能需同时考虑 CLI 接口设计和 Rust 实现成本

## 共享内容与 agent 差异分离

- **日期**：2025-05
- **决策**：`plugin/` 存放跨 agent 共享内容，`targets/<agent>/` 存放差异化配置
- **理由**：避免重复维护相同逻辑；各 agent 的 hook 格式和注册方式不同，需要差异化处理
- **后果**：修改共享内容时需通过 assemble.sh 同步到各 agent 产物

## 多分支 .dev-doc

- **日期**：2025-06
- **决策**：`.dev-doc/<branch>/` 按分支隔离流程文档
- **理由**：支持多条开发线并行，避免分支间流程状态互相干扰
- **后果**：STATUS.yaml、CHANGELOG 等按分支独立管理

## 归档使用 SQLite

- **日期**：2025-06
- **决策**：迭代归档从目录结构迁移到 SQLite 存储
- **理由**：目录归档文件数量增长后查询困难；SQLite 支持结构化查询且单文件易管理
- **后果**：提供 `dow archive migrate` 迁移路径；查询通过 `dow archive` 子命令

## rollback 作为 iterate 的逆操作

- **日期**：2026-06
- **决策**：`dow rollback --version <v>` 从 archive.db 还原已归档的 task/issue/doc 文件，标记该迭代为 rolled back
- **理由**：iterate 后发现问题需要回退时，手动还原文件繁琐且易出错；rollback 提供原子化逆操作
- **后果**：rollback 不撤销 git commit，仅还原流程状态；还原的 task 保持 done_ 前缀，issue 保持 closed_ 前缀；文件 seq 冲突时现有文件顺延

## DEV 阶段临时需求判断靠 prompt 规则

- **日期**：2026-06
- **决策**：DEV 阶段收到用户新消息时，main agent 通过 inject prompt 中的 complexity × relation 规则自行判断处理方式，不加 hook 或强制门控
- **理由**：不想过度工程化；agent 在 DEV 阶段持有最全上下文，适合做这个判断
- **风险**：agent 可能倾向把新需求归为 S+supplement 直接吸收（阻力最小路径）。如实际使用中频繁偏移，再评估加 hook 约束
- **观察指标**：是否出现 task scope 膨胀、done_when 不断追加、files.modify 超出原定范围

## claim agent_id 检测策略

- **日期**：2026-06
- **决策**：claim 记录 agent_id 字段，检测优先级为 `DOW_AGENT_ID` 环境变量 → TTY 路径 (Unix) → caller process ID
- **理由**：多 agent 并发开发时需要区分 claim 归属；Claude Code 环境无 TTY，需 fallback；Windows 无 PPID API
- **后果**：guard hook 对 agent_id 不匹配发出 advisory warning（ask），不强制 block；避免误杀同用户切终端场景

## dashboard 嵌入式前端

- **日期**：2026-06
- **决策**：dashboard 前端资源通过 rust-embed 编译时嵌入二进制，运行时 axum 提供 HTTP 服务 + SSE 推送
- **理由**：零外部依赖部署；VS Code 扩展通过 iframe 嵌入 localhost 页面，复用 100% 前端代码
- **后果**：前端改动需重新编译 dow 二进制；dashboard-frontend/ 纳入 Cargo.toml 的 embed 路径

## task/issue ID 跨文件全局唯一

- **日期**：2026-06
- **决策**：task ID（T001, T002...）和 issue ID（I001, I002...）在所有文件中全局递增，不按文件重新计数
- **理由**：rollback 还原文件后 ID 冲突；claim 操作需要唯一 ID 来定位目标
- **后果**：`dow task create` 创建新 task 时扫描所有 task 文件（含 done_task_）取最大 ID 继续递增；validator 检查全局 1..N 连续性；claim 检测到重复 ID 时报错
