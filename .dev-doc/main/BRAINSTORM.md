# 头脑风暴记录 — dow dashboard

**日期**：2026-06-28

## 背景与目的

当前 dev-flow 的项目状态只能通过 CLI 命令逐个查看（`dow status`、`dow task list`、`dow issue list`），缺少一个全局可视化视图。尤其是 task 之间的依赖关系，在文本形式下难以直观把握。

目标：提供 `dow dashboard` 命令，启动本地 web 服务，以可视化方式展示项目全貌——重点是依赖图，辅以文档查阅和任务/issue 管理。

## 关键决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 实时更新机制 | SSE (Server-Sent Events) | 单向推送足够，浏览器自动重连，比 WebSocket 实现简单，比轮询更实时 |
| 依赖图布局 | Dagre 初始位置 + D3 force 物理模拟 | 分层布局保证结构清晰，弹性拖拽提供交互趣味 |
| 前端资源打包 | rust-embed | 自动嵌入目录、MIME 推断、debug 热更新、gzip 压缩 |
| 进程生命周期 | 前台运行 + 无连接自动退出 | 最简单的资源管理，关 tab 即停 |
| HTTP 框架 | axum | Rust 生态主流，轻量，与 tokio 生态一致 |
| 文件监听 | notify crate | 监听 .dev-doc/ 变化触发 SSE 推送 |

## 设计方案

### 架构

```
dow dashboard
     │
     ├─ axum HTTP server (localhost:随机端口)
     │   ├─ GET /           → 前端 SPA (rust-embed 嵌入)
     │   ├─ GET /api/data   → 完整项目数据 JSON
     │   └─ GET /api/events → SSE 端点（推送变更）
     │
     ├─ notify file watcher
     │   └─ 监听 .dev-doc/ → 触发 SSE 广播
     │
     └─ 连接计数器
         └─ SSE 连接数归零 → 延迟几秒后退出
```

### 组件

**后端 (Rust)**：

1. `commands/dashboard.rs` — 命令入口，启动 server + watcher + 打开浏览器
2. `dashboard/server.rs` — axum 路由、SSE 广播、连接管理
3. `dashboard/data.rs` — 读取 .dev-doc/ 并序列化为统一 JSON 结构
4. `dashboard/watcher.rs` — notify 监听 + 去抖（debounce ~500ms）

**前端 (嵌入的 HTML/JS/CSS)**：

1. `dashboard/frontend/index.html` — SPA 入口
2. `dashboard/frontend/app.js` — 主逻辑，SSE 监听 + 路由切换
3. `dashboard/frontend/graph.js` — Dagre + D3 依赖图渲染
4. `dashboard/frontend/style.css` — 样式

### 页面结构

**Home Tab**：
- 左上：项目状态面板（phase / mode / version / task 进度 / issue 统计）
- 左下：task + issue 列表，点击显示详情
- 右侧：依赖图（全局视图）

**Docs Tab**：
- Markdown 渲染，切换查看 BRAINSTORM.md / PRD.md / SPEC.md
- 不存在的文档显示占位提示

**Tasks Tab**：
- 上方：看板视图（pending / in_progress / done 三列，紧凑卡片）
- 下方：可滚动详情区，按顺序展示每个 task 的完整内容（title / priority / complexity / depends / done_when / files）
- 点击看板卡片跳转到下方对应位置

**Issues Tab**：
- 布局同 Tasks Tab（看板 + 详情滚动区）

### 依赖图规则

- 节点大小 → 复杂度：S(22px) / M(30px) / L(38px) / XL(46px)
- 节点颜色 → 优先级：P0(红 #ff6b6b) / P1(黄 #ffd93d) / P2(绿 #6bcf7f)
- 节点边框 → 状态：done(青 #4ecdc4) / in_progress(蓝 #a0c4ff) / pending(灰 #555)
- 箭头方向 → from 依赖 to（箭头从依赖方指向被依赖方）
- 交互：弹性拖拽（松手弹回 Dagre 位置）+ 点击聚焦（高亮依赖链，淡化无关节点）

### 数据流

```
.dev-doc/ 文件变化
  → notify 检测
  → debounce 500ms
  → 重新读取 data.rs
  → SSE 广播 JSON
  → 浏览器接收
  → 前端重渲染（图增量更新，不重置拖拽状态）
```

### 错误处理

- 端口被占用：自动尝试下一个端口（范围 9800-9900）
- .dev-doc/ 不存在：提示用户先运行 `dow init`
- 浏览器打开失败：打印 URL 提示手动访问
- SSE 连接断开：浏览器 EventSource 自动重连

## 约束与边界

- 只读展示，不支持从 web 端修改数据（操作仍通过 CLI）
- 仅服务 localhost，不暴露到网络
- 前端不使用构建工具（无 npm/webpack），纯 vanilla JS + CDN 引用 D3/Dagre
- 不缓存数据，每次推送都是完整快照（.dev-doc 数据量小，简化实现）

## 下一步

功能相对明确，建议直接进入 `/spec` 进行技术规格设计。
