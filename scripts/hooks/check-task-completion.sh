#!/bin/bash
# Hook: PostToolUse(Edit|Write) on task/ 和 issue/ 文件
# 检测 task 完成度，自动重命名 done_/closed_ 前缀，触发 devtest 提醒

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# 确定文档根目录
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.yaml" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 只在 DEV 阶段触发
PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
[ "$PHASE" != "DEV" ] && exit 0

# === task/ 目录统计 ===
TOTAL=0; DONE=0
for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL=$((TOTAL + ${CNT:-0}))
  CNT=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; DONE=$((DONE + ${CNT:-0}))
done

[ "$TOTAL" -eq 0 ] && exit 0

UNDONE=$((TOTAL - DONE))

if [ "$UNDONE" -eq 0 ]; then
  echo "[dev-flow] 所有任务已完成（$DONE/$TOTAL）。"
  echo "→ 立即执行 /test 进行全量验证。"
else
  # 检查是否有刚完成的任务
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    LAST_CHECKED=$(grep -n "^- \[x\]" "$f" | tail -1 | cut -d: -f1)
    if [ -n "$LAST_CHECKED" ]; then
      TASK_NAME=$(sed -n "${LAST_CHECKED}p" "$f" | sed 's/^- \[x\] //' | sed 's/（.*//;s/(.*$//')
      if [ -n "$TASK_NAME" ]; then
        echo "[dev-flow] 任务完成（$DONE/$TOTAL）：$TASK_NAME"
        echo "→ 自动触发 /devtest。立即对该任务执行例行测试，不需要询问用户。"
        break
      fi
    fi
  done
fi

# === done_ 前缀自动重命名（task 文件全部勾选时） ===
for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  FILE_TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; FILE_TOTAL=${FILE_TOTAL:-0}
  FILE_DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; FILE_DONE=${FILE_DONE:-0}
  if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
    BASENAME=$(basename "$f")
    NEWNAME="$DOC_ROOT/task/done_$BASENAME"
    if [ ! -f "$NEWNAME" ]; then
      mv "$f" "$NEWNAME"
      echo "[dev-flow] 批次全部完成，已标记：done_$BASENAME"
    fi
  fi
done

# === closed_ 前缀自动重命名（issue 文件全部勾选时） ===
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    FILE_TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; FILE_TOTAL=${FILE_TOTAL:-0}
    FILE_DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; FILE_DONE=${FILE_DONE:-0}
    if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
      BASENAME=$(basename "$f")
      NEWNAME="$DOC_ROOT/issue/closed_$BASENAME"
      if [ ! -f "$NEWNAME" ]; then
        mv "$f" "$NEWNAME"
        echo "[dev-flow] Issue 全部关闭：closed_$BASENAME"
      fi
    fi
  done
fi
