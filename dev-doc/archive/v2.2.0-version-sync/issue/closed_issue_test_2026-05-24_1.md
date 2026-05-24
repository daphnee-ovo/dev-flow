---
source: test
nums: 1
---

- [x] I1：iterate.sh P0 issue 检测逻辑误判（已修复的 P0 仍阻断迭代）
  - severity: P1
  - location：scripts/commands/iterate.sh:51-56
  - description：iterate.sh 检查 P0 issue 时，使用 `grep -q "severity: P0"` 扫描所有 `issue_*.md` 文件。该逻辑不区分条目是 `[ ]`（未关闭）还是 `[x]`（已修复）。当一个 issue 文件中包含已修复的 P0 条目（`[x]`）和未修复的非 P0 条目（`[ ]`）时，文件不会被 rename 为 `closed_*`（因为有未关闭条目），但 iterate.sh 仍因检测到 "severity: P0" 而错误阻断迭代。
  - reproduce：1. 创建 issue 文件 `issue_test_xxx.md`，包含一个 `- [x]` P0 条目和一个 `- [ ]` P1 条目；2. 确保所有 task 完成；3. 运行 `DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "topic"`；4. 观察到错误退出并报 "P0 issue" 未关闭，但实际 P0 已修复。
  - fix：改为逐条解析 issue 条目状态，只有未关闭的 P0 才计入阻断
