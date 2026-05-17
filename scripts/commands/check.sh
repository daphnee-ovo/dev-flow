#!/bin/bash
# /check 命令脚本化实现
# 用法：bash check.sh [DOC_ROOT]

DOC_ROOT="${1:-dev-doc}"

# 检测多工程模式
if find "$DOC_ROOT" -maxdepth 2 -name "STATUS.yaml" -path "*/*/STATUS.yaml" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  if [ -n "$BRANCH" ] && [ -f "$DOC_ROOT/$BRANCH/STATUS.yaml" ]; then
    DOC_ROOT="$DOC_ROOT/$BRANCH"
  fi
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  echo "[dev-flow] STATUS.yaml 不存在"
  exit 1
fi

PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
WARNINGS=()
OK=()

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
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[ \]" "$f" 2>/dev/null) || true; OPEN_ISSUES=$((OPEN_ISSUES + ${CNT:-0}))
  done
fi
if [ "$OPEN_ISSUES" -gt 0 ] && [ "$PHASE" = "DONE" ]; then
  WARNINGS+=("阶段为 DONE 但仍有 $OPEN_ISSUES 个未关闭 issue")
fi
if [ "$OPEN_ISSUES" -eq 0 ] && [ -d "$DOC_ROOT/issue" ]; then
  OK+=("所有 issue 已关闭")
fi

# === 4. 代码变更 vs 文档更新时间 ===
UPDATED=$(grep "^updated:" "$STATUS_FILE" | sed 's/^updated: *//')
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

if [ ${#OK[@]} -gt 0 ]; then
  echo "✓ 正常（${#OK[@]}项）："
  for o in "${OK[@]}"; do
    echo "  - $o"
  done
fi

if [ ${#WARNINGS[@]} -eq 0 ]; then
  echo "✓ 文档同步状态良好，无需操作。"
fi
