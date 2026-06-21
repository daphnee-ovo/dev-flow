---
source: test
nums: 1
---

- [x] ISSUE-I001：Build dow binaries 部署阶段缺少 scripts/bin
  - severity: P1
  - location：.github/workflows/build-dow.yml:61
  - description：GitHub Actions `Build dow binaries` 的 `commit-binaries` job 在 clean checkout 中执行 `cp ... scripts/bin/`，但仓库没有追踪 `scripts/bin/` 目录，也没有 `scripts/bin/dow-wrapper`，导致部署阶段失败。
  - reproduce：推送包含 `dow/**` 变更到 `main`，或手动运行 `Build dow binaries` workflow。
  - fix：workflow 部署阶段创建 `scripts/bin`，直接生成 `scripts/bin/dow` wrapper，并授予 `contents: write` 以允许提交生成的二进制。
