---
source: other
nums: 1
---

- [x] ISSUE-I004：dashboard 看板增加 in_progress 状态（读取 claim.lock）+ 依赖图闪烁标识
  - severity: P1
  - location：dow/src/dashboard/data.rs:214
  - description：task/issue 看板缺少 in_progress 列，需读取 claim.lock 判定；依赖图中 in_progress 节点需闪烁动画标识
  - reproduce：dow claim T001 后打开 dashboard，该任务仍显示在 pending 列而非 in_progress
  - fix：data.rs 读取 claim.lock 活跃 claims，将 pending task 和 open issue 标记为 in_progress；views.js issue 看板增加 In Progress 列 + task/issue filter 增加 Active 按钮；graph.js 对 in_progress 节点添加 node-pulse class；style.css 添加脉冲动画
  - files_modify: [dow/src/dashboard/data.rs, dow/dashboard-frontend/views.js, dow/dashboard-frontend/graph.js, dow/dashboard-frontend/style.css]
  - files_create: []
