---
source: other
nums: 1
---

- [x] ISSUE-I014：guard hook: .claude/ 路径下代码文件应走 ask 而非直接放行
  - severity: P1
  - location：
  - description：is_ai_config whitelist 无条件放行 .claude/ 全部文件，但 .claude/skills/ 下可能包含业务代码
  - reproduce：
  - fix：
