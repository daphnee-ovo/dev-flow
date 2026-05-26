---
source: test
created: 2026-05-26
---

- [x] iterate.sh audit 模式恢复后输出显示旧 mode 值
  - severity: P2
  - location: scripts/commands/iterate.sh:240
  - description: iterate 完成后最后的摘要输出 `echo "模式：$MODE"` 使用的是迭代开始时读取的原始值（如 `audit/quick`），而非恢复后的实际模式（如 `quick`）。STATUS.yaml 已正确恢复，仅输出信息有误导。
  - expected: audit 模式 iterate 完成后输出应显示恢复后的模式（如 `模式：quick`）
  - actual: 输出显示 `模式：audit/quick`（迭代前的旧值）
  - reproduce: |
      设置 STATUS.yaml mode=audit/quick phase=DEV，执行 DEVFLOW_NO_CONFIRM=1 bash iterate.sh "fix" "patch"，
      观察最后一行输出仍为 `模式：audit/quick` 而非 `模式：quick`
  - fix: 将 echo "$MODE" 改为 echo "$(devflow_yaml_get "$STATUS_FILE" mode)"，读取恢复后的实际值
