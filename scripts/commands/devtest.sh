#!/bin/bash
# /devtest 最小闭环：PASS / FAIL / NEEDS_CONTEXT
# 用法：
#   bash devtest.sh [DOC_ROOT]
#   bash devtest.sh --continuous [DOC_ROOT]
#   bash devtest.sh --step [DOC_ROOT]
#   bash devtest.sh --result PASS|FAIL|NEEDS_CONTEXT [DOC_ROOT]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

RESULT="PASS"
DOC_ROOT="dev-doc"

while [ $# -gt 0 ]; do
  case "$1" in
    --continuous)
      DOC_ROOT=$(devflow_resolve_doc_root "${2:-dev-doc}")
      STATUS_FILE="$DOC_ROOT/STATUS.yaml"
      [ -f "$STATUS_FILE" ] || { echo "[dev-flow] STATUS.yaml 不存在：$STATUS_FILE"; exit 1; }
      devflow_yaml_set "$STATUS_FILE" exec_mode "continuous"
      echo "[dev-flow] devtest exec_mode: continuous"
      exit 0
      ;;
    --step)
      DOC_ROOT=$(devflow_resolve_doc_root "${2:-dev-doc}")
      STATUS_FILE="$DOC_ROOT/STATUS.yaml"
      [ -f "$STATUS_FILE" ] || { echo "[dev-flow] STATUS.yaml 不存在：$STATUS_FILE"; exit 1; }
      devflow_yaml_set "$STATUS_FILE" exec_mode "step"
      echo "[dev-flow] devtest exec_mode: step"
      exit 0
      ;;
    --result)
      RESULT="$2"
      shift 2
      ;;
    *)
      DOC_ROOT="$1"
      shift
      ;;
  esac
done

DOC_ROOT=$(devflow_resolve_doc_root "$DOC_ROOT")
STATUS_FILE="$DOC_ROOT/STATUS.yaml"
[ -f "$STATUS_FILE" ] || { echo "[dev-flow] STATUS.yaml 不存在：$STATUS_FILE"; exit 1; }

PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)
if [ "$PHASE" != "DEV" ]; then
  echo "[dev-flow] devtest 只能在 DEV 阶段执行，当前阶段：${PHASE:-未知}"
  exit 1
fi

case "$RESULT" in
  PASS|FAIL|NEEDS_CONTEXT) ;;
  *) echo "[dev-flow] 无效 devtest result：$RESULT"; exit 1 ;;
esac

TASK_FILE=""
TASK_LINE=""
TASK_TITLE=""

for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  line_no=$(grep -n '^- \[x\]' "$f" | tail -1 | cut -d: -f1)
  if [ -n "$line_no" ]; then
    TASK_FILE="$f"
    TASK_LINE="$line_no"
    TASK_TITLE=$(sed -n "${line_no}p" "$f" | sed 's/^- \[x\] *//')
  fi
done

if [ -z "$TASK_FILE" ]; then
  echo "[dev-flow] 未找到已勾选但待 devtest 的任务"
  exit 1
fi

write_issue() {
  local title="$1"
  local detail="$2"
  local date seq file
  date=$(date +%Y-%m-%d)
  mkdir -p "$DOC_ROOT/issue"
  seq=$(find "$DOC_ROOT/issue" -name "issue_devtest_${date}_*.md" -o -name "closed_issue_devtest_${date}_*.md" 2>/dev/null \
    | awk -F_ -v date="$date" '{ n=$NF; sub(/\.md$/, "", n); if (n+0 > max) max=n+0 } END { print max+1 }')
  file="$DOC_ROOT/issue/issue_devtest_${date}_${seq}.md"
  cat > "$file" << EOF
---
source: devtest
nums: 1
---

- [ ] ISSUE-I001: $title
  - severity: P1
  - source: devtest
  - refs: $TASK_TITLE
  - location: $TASK_FILE:$TASK_LINE
  - current: $detail
  - expected: task 通过 devtest 后才能保持完成
  - reproduce: bash scripts/commands/devtest.sh --result FAIL
  - root_cause:
  - fix:
  - close_when: 重新执行 devtest 返回 PASS
EOF
  echo "$file"
}

case "$RESULT" in
  PASS)
    echo "[dev-flow] devtest PASS：$TASK_TITLE"
    ;;
  NEEDS_CONTEXT)
    echo "[dev-flow] devtest NEEDS_CONTEXT：$TASK_TITLE"
    echo "→ 信息不足，保持任务状态，不继续推进。"
    exit 2
    ;;
  FAIL)
    tmp="${TASK_FILE}.tmp.$$"
    awk -v target="$TASK_LINE" 'NR == target { sub(/^- \[x\]/, "- [ ]") } { print }' "$TASK_FILE" > "$tmp" && mv "$tmp" "$TASK_FILE"
    issue_file=$(write_issue "devtest 未通过：$TASK_TITLE" "devtest 返回 FAIL")
    echo "[dev-flow] devtest FAIL：$TASK_TITLE"
    echo "→ 已取消任务勾选并写入 issue：$issue_file"
    exit 1
    ;;
esac

read -r TOTAL DONE << EOF
$(devflow_count_tasks "$DOC_ROOT")
EOF

if [ "$TOTAL" -gt 0 ] && [ "$DONE" -eq "$TOTAL" ]; then
  echo "→ 所有任务已完成，下一步执行 /test"
else
  echo "→ 继续下一个任务"
fi

exit 0
