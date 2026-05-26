#!/bin/bash
# /check 命令脚本化实现
# 用法：bash check.sh [DOC_ROOT]

DOC_ROOT="${1:-dev-doc}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

INPUT_DOC_ROOT="$DOC_ROOT"
DOC_ROOT=$(devflow_resolve_doc_root "$DOC_ROOT")

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  echo "[dev-flow] STATUS.yaml 不存在"
  exit 1
fi

PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)
MODE=$(devflow_yaml_get "$STATUS_FILE" mode)
ERRORS=()
WARNINGS=()
OK=()

# === 0. doc-root 检查 ===
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "$INPUT_DOC_ROOT/$BRANCH/STATUS.yaml" ] && [ "$DOC_ROOT" != "$INPUT_DOC_ROOT/$BRANCH" ]; then
  ERRORS+=("doc_root_mismatch：当前分支 $BRANCH 应使用 $INPUT_DOC_ROOT/$BRANCH")
else
  OK+=("当前文档根：$DOC_ROOT")
fi

# === 1. CHANGELOG 检查 ===
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  if [ -s "$DOC_ROOT/CHANGELOG.md" ]; then
    OK+=("CHANGELOG.md 存在且非空")
  else
    WARNINGS+=("CHANGELOG.md 为空，尚未有会话记录")
  fi
else
  WARNINGS+=("CHANGELOG.md 不存在，save-changelog hook 可能未触发")
fi

# === 2. task/ 完成度与 phase 匹配 ===
TOTAL=0; DONE=0
for f in "$DOC_ROOT/task/task_"*.md "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  DECLARED=$(awk -F: '/^nums:/ { gsub(/[[:space:]]/, "", $2); print $2; exit }' "$f")
  ACTUAL=$(grep -c "^- \\[[ x]\\]" "$f" 2>/dev/null) || ACTUAL=0
  if [ -n "$DECLARED" ] && [ "$DECLARED" != "$ACTUAL" ]; then
    ERRORS+=("task_nums_mismatch：$f 声明 nums=$DECLARED，实际任务数=$ACTUAL")
  fi
  CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL=$((TOTAL + ${CNT:-0}))
  CNT=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; DONE=$((DONE + ${CNT:-0}))
done

if [ "$PHASE" = "DEV" ] && [ "$TOTAL" -eq 0 ]; then
  WARNINGS+=("阶段为 DEV 但 task/ 目录无任务文件")
fi
if [ "$TOTAL" -gt 0 ] && [ "$DONE" -eq "$TOTAL" ] && [ "$PHASE" = "DEV" ]; then
  WARNINGS+=("所有任务已完成但阶段仍为 DEV，建议执行 /test")
fi

# === 3. issue/ 状态检查 ===
OPEN_ISSUES=0
OPEN_P0=0
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[ \]" "$f" 2>/dev/null) || true; OPEN_ISSUES=$((OPEN_ISSUES + ${CNT:-0}))
    IN_OPEN=false
    while IFS= read -r line; do
      if echo "$line" | grep -qE "^- \[ \]"; then
        IN_OPEN=true
      elif echo "$line" | grep -qE "^- \[x\]"; then
        IN_OPEN=false
      elif [ "$IN_OPEN" = true ] && echo "$line" | grep -qE "severity:[[:space:]]*P0"; then
        OPEN_P0=$((OPEN_P0 + 1))
        IN_OPEN=false
      fi
    done < "$f"
  done
fi
if [ "$OPEN_P0" -gt 0 ]; then
  ERRORS+=("open_p0_issue：有 $OPEN_P0 个未关闭 P0 issue")
fi
if [ "$OPEN_ISSUES" -gt 0 ] && [ "$PHASE" = "DONE" ]; then
  WARNINGS+=("阶段为 DONE 但仍有 $OPEN_ISSUES 个未关闭 issue")
fi
if [ "$OPEN_ISSUES" -eq 0 ] && [ -d "$DOC_ROOT/issue" ]; then
  OK+=("所有 issue 已关闭")
fi

# === 4. 代码变更 vs 文档更新时间 ===
UPDATED=$(devflow_yaml_get "$STATUS_FILE" updated)
LAST_COMMIT_TIME=$(git log -1 --format="%ai" 2>/dev/null | cut -d' ' -f1,2 | cut -c1-16)
if [ -n "$LAST_COMMIT_TIME" ] && [ -n "$UPDATED" ]; then
  # 简单比较日期部分
  COMMIT_DATE=$(echo "$LAST_COMMIT_TIME" | cut -d' ' -f1)
  STATUS_DATE=$(echo "$UPDATED" | cut -d' ' -f1)
  if [[ "$COMMIT_DATE" > "$STATUS_DATE" ]]; then
    WARNINGS+=("最近代码提交($COMMIT_DATE)晚于 STATUS 更新($STATUS_DATE)，文档可能未同步")
  else
    OK+=("STATUS 更新时间与代码同步")
  fi
fi

# === 5. 阶段必要文件检查 ===
case "$PHASE" in
  SPEC|TASK|DEV|TEST|DONE)
    [ ! -f "$DOC_ROOT/SPEC.md" ] && WARNINGS+=("阶段为 $PHASE 但缺少 SPEC.md")
    ;;
esac

# === 6. SPEC 轻量验收检查 ===
if [ -f "$DOC_ROOT/SPEC.md" ]; then
  if ! grep -qE "SPEC-AC-|## Acceptance|## 5\\. 验收契约|验收" "$DOC_ROOT/SPEC.md"; then
    case "$MODE" in
      full|quick) ERRORS+=("spec_missing_ac：$MODE 模式 SPEC 缺少可测验收") ;;
      *) WARNINGS+=("SPEC 缺少明确验收，建议补充 Acceptance") ;;
    esac
  fi
fi

# === 7. TEST 报告检查 ===
if [ "$TOTAL" -gt 0 ] && [ "$DONE" -eq "$TOTAL" ]; then
  if [ ! -f "$DOC_ROOT/TEST.md" ]; then
    WARNINGS+=("所有任务已完成但缺少 TEST.md")
  elif grep -qE "FAILED SUITES:|FAIL: [1-9][0-9]*|失败: [1-9][0-9]*|失败：[1-9][0-9]*|未通过：[1-9][0-9]*|未通过: [1-9][0-9]*" "$DOC_ROOT/TEST.md"; then
    WARNINGS+=("TEST.md 记录了未通过测试，建议继续 /test 或 /fix")
  fi
fi
case "$PHASE" in
  DEV|TEST|DONE)
    if [ "$TOTAL" -eq 0 ]; then
      WARNINGS+=("阶段为 $PHASE 但 task/ 目录无任务")
    fi
    ;;
esac

# === 输出 ===
echo "[dev-flow] 文档同步检查"
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "当前阶段：$PHASE"
echo ""

if [ ${#WARNINGS[@]} -gt 0 ]; then
  echo "⚠ 需要关注（${#WARNINGS[@]}项）："
  for w in "${WARNINGS[@]}"; do
    echo "  - $w"
  done
  echo ""
fi

if [ ${#ERRORS[@]} -gt 0 ]; then
  echo "✗ 阻断错误（${#ERRORS[@]}项）："
  for e in "${ERRORS[@]}"; do
    echo "  - $e"
  done
  echo ""
fi

if [ ${#OK[@]} -gt 0 ]; then
  echo "✓ 正常（${#OK[@]}项）："
  for o in "${OK[@]}"; do
    echo "  - $o"
  done
fi

if [ ${#WARNINGS[@]} -eq 0 ] && [ ${#ERRORS[@]} -eq 0 ]; then
  echo "✓ 文档同步状态良好，无需操作。"
fi

[ ${#ERRORS[@]} -gt 0 ] && exit 1
exit 0
