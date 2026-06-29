---
source: other
nums: 1
---

- [x] ISSUE-I001：Dashboard 图节点 hover 闪烁 + task 列表跳动
  - severity: P1
  - location：
  - description：光标悬浮在图节点上时 tooltip 闪烁；Home 左下 task 列表项一直跳动。原因：D3 force simulation 持续运动导致节点 transform 不断变化，触发 mouseenter/mouseleave 循环；SSE 每次推送触发完整 re-render 导致列表重建
  - reproduce：
  - fix：
