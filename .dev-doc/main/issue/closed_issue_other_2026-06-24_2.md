---
source: other
nums: 2
---

- [x] ISSUE-I011：CLAUDE.md Hooks 段缺少 PostToolUse(Bash) 触发器
  - severity: P2
  - location：CLAUDE.md:92
  - description：Hooks 段只列了 PostToolUse(Write|Edit)，但实际 hooks.json 还注册了 PostToolUse(Bash) → dow hooks post-bash（检测分支切换）。文档遗漏该触发器。
  - fix：在 CLAUDE.md Hooks 段补充 PostToolUse(Bash) → dow hooks post-bash 条目

- [x] ISSUE-I012：docs/structure.md 目录树缺少 npm/ 和 scripts/
  - severity: P2
  - location：docs/structure.md:34
  - description：实际顶层目录有 npm/（npm 包装）和 scripts/（工具脚本），但 structure.md 目录树未列出。
  - fix：在 structure.md 目录树补充 scripts/ 和 npm/ 两个条目

