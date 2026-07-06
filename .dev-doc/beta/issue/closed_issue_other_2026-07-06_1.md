---
source: other
nums: 1
---

- [x] ISSUE-I001：CI build-dow.yml 移除 commit-binaries job（scripts/bin 已在 gitignore）
  - severity: P1
  - location：.github/workflows/build-dow.yml:50
  - description：commit-binaries 尝试 git add scripts/bin/dow* 但该路径已在 .gitignore，导致 CI 失败
  - reproduce：push 到 main 触发 build-dow workflow
  - fix：移除 commit-binaries job，仅保留 build + upload artifact
  - files_modify: [.github/workflows/build-dow.yml]
  - files_create: []
