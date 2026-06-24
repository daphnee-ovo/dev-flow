---
source: other
nums: 1
---

- [x] ISSUE-I013：CLAUDE.md 缺少禁止在 dow/ 下创建 .dev-doc 的说明
  - severity: P2
  - location：CLAUDE.md:5
  - description：在 dow/ 目录内运行 dow 命令会生成 dow/.dev-doc/ 污染源码树。注意事项段未明确禁止，导致已发生一次污染。
  - reproduce：cd dow && dow init → 生成 dow/.dev-doc/
  - fix：清理 dow/.dev-doc/ 污染；在 CLAUDE.md 注意事项段补充禁止在 dow/ 下运行 dow/创建 .dev-doc 的说明

