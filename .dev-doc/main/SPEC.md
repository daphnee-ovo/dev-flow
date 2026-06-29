# SPEC: dow dashboard

## Goal

为 dow 添加 `dashboard` 子命令，启动本地 HTTP 服务在浏览器中可视化展示当前项目的流程状态、依赖图和文档内容。通过 SSE 实时推送 .dev-doc/ 的变化，所有浏览器连接断开后自动退出。

## Design

### 新增依赖

```toml
# Cargo.toml [dependencies] 新增
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "fs", "time"] }
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
rust-embed = { version = "8", features = ["compression"] }
notify = { version = "7", features = ["macos_kqueue"] }
notify-debouncer-mini = "0.5"
open = "5"
mime_guess = "2"
tokio-stream = "0.1"
```

### 模块结构

```
dow/src/
├─ commands/
│  └─ dashboard.rs        # 子命令入口：解析参数、协调启动
└─ dashboard/
   ├─ mod.rs              # 模块 re-export
   ├─ server.rs           # axum 路由、SSE 广播、连接计数、自动退出
   ├─ data.rs             # 读取 .dev-doc/ → 统一 JSON 结构
   └─ watcher.rs          # notify 监听 + debounce → 触发广播

dow/dashboard-frontend/   # rust-embed 嵌入目录
├─ index.html             # SPA 入口
├─ app.js                 # Tab 路由 + SSE 监听 + 状态管理
├─ graph.js               # Dagre + D3 force 依赖图
├─ vendor/
│  ├─ d3.v7.min.js        # 本地 vendor（无 CDN 依赖）
│  └─ dagre.min.js
├─ vendor/
│  ├─ highlight.min.js    # 代码高亮（yaml/json/bash/rust）
│  └─ inter.woff2         # Inter 字体
└─ style.css
```

### API 设计

| 端点 | 方法 | 响应 |
|------|------|------|
| `/` | GET | index.html (SPA) |
| `/assets/*` | GET | 前端静态资源 (rust-embed) |
| `/api/data` | GET | 完整项目数据 JSON |
| `/api/events` | GET | SSE 流，事件类型 `update`，data 为完整 JSON |

### 数据结构 (`/api/data` 响应)

```json
{
  "status": { "phase": "DEV", "mode": "fast", "version": "0.2.3", ... },
  "tasks": [
    { "id": "TASK-T001", "title": "...", "priority": "P0", "complexity": "L",
      "status": "in_progress", "depends_on": ["TASK-T002"], "done_when": [...], "files": {...} }
  ],
  "issues": [
    { "id": "ISSUE-001", "title": "...", "severity": "P1", "status": "open", ... }
  ],
  "docs": {
    "brainstorm": { "exists": true, "content": "..." },
    "prd": { "exists": false, "content": null },
    "spec": { "exists": true, "content": "..." }
  }
}
```

### Runtime 集成策略

仅在 `commands/dashboard.rs` 内创建 tokio runtime（隔离式），不改变全局 main 签名，其他子命令保持同步执行不受影响。

### 生命周期

1. `dow dashboard [--port PORT]` 启动
2. 创建 tokio runtime（仅此命令使用）
3. 选择端口：指定端口或自动扫描 9800-9900 找可用端口
4. 启动 axum server + notify watcher
5. 自动打开浏览器（`open` crate），失败则打印 URL
6. SSE 连接计数：连接 +1，断开 -1
7. 计数归零后启动 5 秒倒计时，5 秒内无新连接则退出
8. Ctrl+C 直接退出
9. 多实例：允许同项目多次运行（各绑独立端口，共享同一 .dev-doc/ 数据源）

### 前端关键行为

- SSE 推送完整 JSON 快照，前端按 `id` 字段做 diff：
  - 新增节点：加入 D3 force simulation
  - 删除节点：移除
  - 已有节点：仅更新 data（title/status/priority），不改坐标和拖拽偏移
  - D3 force simulation 不重启，只调用 `.nodes()` / `.links()` 更新数据
- Markdown 渲染使用 marked.js（vendor 嵌入，~50KB），启用 sanitize 防止 XSS
- 无外部网络依赖，全部资源本地嵌入

### UI 设计规范

**风格参考**：Liquid Glass（参考 ui-ux-pro-max luxury-ecommerce demo）— 保留毛玻璃面板质感、柔和光影层次、精致圆角，但适配 Light theme + 粉橙暖色调。克制使用 glass 效果：面板用 `backdrop-filter: blur(12px)` + 半透明白底，不做 morphing 动画和 chromatic aberration（避免与依赖图交互冲突）。

**视觉基础（Light · 樱花奶茶 × Liquid Glass）**：

| Token | 值 | 用途 |
|-------|-----|------|
| `--color-background` | `#FFFBF9` | 页面底色 |
| `--color-surface` | `#FFFFFF` | 卡片/面板背景 |
| `--color-surface-alt` | `#FFF7F5` | 次级面板/hover 态/inactive tab |
| `--color-accent-pink` | `#F2A0B0` | 主题粉（樱花） |
| `--color-accent-orange` | `#F0B078` | 主题橙（奶茶） |
| `--color-text` | `#3A2F2D` | 主文本 |
| `--color-text-muted` | `#A89490` | 辅助文本 |
| `--color-border` | `#F4E8E4` | 边框/分割线 |
| `--color-p0` | `#E85D6F` | P0 紧急 |
| `--color-p1` | `#E8A44C` | P1 正常 |
| `--color-p2` | `#6DC08A` | P2 低 |
| `--color-shadow` | `rgba(242,160,176,0.08)` | 面板阴影（带粉色调） |

- 字体：Inter（vendor 嵌入 woff2），fallback 系统 sans-serif
- 字号：body 14px, heading 18px/24px，代码块 13px monospace
- 间距：8px 基础单位（8/16/24/32）
- 圆角：卡片 12px，按钮/badge 6px，全圆 999px 用于 tag
- 面板阴影：`0 2px 8px var(--color-shadow)`
- 渐变应用：仅依赖图高亮路径可用粉橙渐变，其他元素不使用渐变

**依赖图节点（Network Graph 规范）**：

- 节点数 ≤50：SVG 渲染（当前约束范围内）
- 边：`#D4C4BE` 60% opacity，高亮路径 `#F2A0B0`
- 节点 hover：显示 tooltip（title + priority + status）
- 交互状态：hover 放大 1.05x + soft shadow，active 无布局偏移
- 无障碍：提供 task 列表作为图的等效替代（Tasks tab 即是）

**Tab 导航**：

- 当前 tab 用 `--color-accent-pink` 下划线 + 主文本色，inactive 用 muted 文字无下划线
- Tab 切换更新 URL hash（`#home` / `#docs` / `#tasks` / `#issues`）支持 deep linking
- 键盘可达：Tab 聚焦，Enter 切换，左右箭头在 tab 间移动

**看板卡片**：

- 左侧 3px 色条表示优先级（P0 `#E85D6F` / P1 `#E8A44C` / P2 `#6DC08A`）
- 背景：对应优先级的极浅底色（`#FFF5F6` / `#FFF8F2` / `#F2FBF5`）
- hover：`translateY(-2px)` + `box-shadow: 0 4px 12px var(--color-shadow)`，150ms ease-out
- 非拖拽：无拖拽把手，无排序暗示，明确只读
- 点击：smooth scroll 到下方详情区对应位置

**Markdown 渲染区**：

- 代码块语法高亮（内嵌 highlight.js 最小子集，仅 yaml/json/bash/rust）
- heading 层级：h1-h3 有明确视觉区分
- 链接：`--color-accent` + underline on hover

**响应式**：

- 最小适配宽度 1024px（开发者工具场景，不考虑移动端）
- Home 页左侧面板 280px 固定，右侧图区自适应
- 窗口过小时左侧面板可折叠

**动画与性能**：

- 仅使用 transform/opacity 动画，避免触发 reflow
- 图节点 transition 150-300ms，spring physics 弹性
- SSE 重连状态：顶部显示 "Reconnecting..." 指示条
- `prefers-reduced-motion`：禁用 force simulation 弹性，直接定位

### 约束

- 只读：不提供修改 .dev-doc/ 的 API
- 仅绑定 127.0.0.1，不暴露网络
- 适用规模：≤50 task（超出不做特殊优化，但不崩溃）
- 多实例隔离：每个 `dow dashboard` 绑定独立端口 + 独立 .dev-doc/ 路径
- 最小浏览器：支持 ES2020+（Chrome 80+, Firefox 80+, Safari 14+）

## Acceptance

- SPEC-AC-001: `dow dashboard` 命令启动后在 2 秒内打开浏览器，显示项目状态和依赖图
- SPEC-AC-002: 在 dashboard 运行期间通过 CLI 修改 task（如 `dow task done`），浏览器在 1 秒内反映变化
- SPEC-AC-003: 关闭所有浏览器 tab 后，dow 进程在 5-8 秒内自动退出
- SPEC-AC-004: 依赖图节点可弹性拖拽，松手后弹回分层位置；点击节点高亮其依赖链
- SPEC-AC-005: Docs tab 能正确渲染 BRAINSTORM.md / PRD.md / SPEC.md 的 Markdown 内容
- SPEC-AC-006: Tasks/Issues tab 以看板 + 滚动详情形式展示，点击看板卡片跳转到对应详情
- SPEC-AC-007: 无外部网络依赖，断网环境下功能完整

## Test Plan

- 单元测试：`data.rs` 数据读取 + 序列化（mock .dev-doc/ 目录）
- 集成测试：启动 server → HTTP 请求 `/api/data` → 验证 JSON 结构
- 手动验证：启动 dashboard → 修改 task → 观察浏览器实时更新 → 关闭 tab → 确认进程退出

## Self Check

- [x] Goal is clear
- [x] Acceptance criteria are testable
- [x] Matches current mode (fast: 有 Design 但精简，无 Requirements Trace)
- [x] 新增依赖已列出
- [x] 模块边界清晰
- [x] 无外部网络依赖约束已覆盖
