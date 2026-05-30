---
source: other
nums: 1
---

- [ ] ISSUE-I001：fix skill 不应自动 bump version
  - severity: P1
  - location：plugin/skills/dev-flow/fix.md
  - description：/fix skill 指令中规定"P0 issue 关闭时自动 bump minor"，但 version bump 应统一由 /iterate 流程处理，fix 阶段自动 bump 会导致版本号与实际交付不对齐
  - fix：

