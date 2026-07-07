---
source: other
nums: 1
---

- [x] ISSUE-I003：dashboard 显示用户内容时未做 HTML 转义，含 HTML 标签的 description 破坏渲染
  - severity: P1
  - location：dow/dashboard-frontend/views.js:282
  - description：issue description 含 <script> 等 HTML 标签时通过 innerHTML 直接插入 DOM，浏览器将其解析为真实标签导致后续内容不显示
  - reproduce：slides-report-helen 项目 ISSUE-I062 的 description 含 <script type=text/slides>，打开 dashboard Issues 视图后该 issue 及后续项目不显示
  - fix：添加 esc() HTML 转义函数，对 views.js 和 graph.js 中所有用户内容（title/description/refs/done_when）做转义后再插入 innerHTML
  - files_modify: [dow/dashboard-frontend/views.js]
  - files_create: []
