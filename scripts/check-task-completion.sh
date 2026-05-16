#!/bin/bash
# Hook: PostToolUse(Edit) on TASK.md
# 检测 TASK.md 是否有新勾选的任务，强制触发例行测试提醒
# 输出非空内容 = Claude 收到提醒

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# 确定文档根目录
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.md" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

TASK_FILE="$DOC_ROOT/TASK.md"
STATUS_FILE="$DOC_ROOT/STATUS.md"

if [ ! -f "$TASK_FILE" ] || [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 只在 DEV 阶段触发
PHASE=$(grep "当前阶段" "$STATUS_FILE" | sed 's/.*：//' | tr -d ' ')
if [ "$PHASE" != "DEV" ]; then
  exit 0
fi

# 检查是否所有任务都已完成
TOTAL=$(grep -c "^- \[" "$TASK_FILE" 2>/dev/null || echo 0)
DONE=$(grep -c "^- \[x\]" "$TASK_FILE" 2>/dev/null || echo 0)
UNDONE=$((TOTAL - DONE))

if [ "$TOTAL" -eq 0 ]; then
  exit 0
fi

if [ "$UNDONE" -eq 0 ]; then
  # 全部完成 → 强制进入项目 TEST
  echo "[dev-flow] 所有任务已完成（$DONE/$TOTAL）。"
  echo "→ 立即执行 /test 进行全量验证。禁止跳过，禁止先向用户报告完成。"
else
  # 有新勾选 → 强制触发 devtest
  LAST_CHECKED=$(grep -n "^- \[x\]" "$TASK_FILE" | tail -1 | cut -d: -f1)
  if [ -n "$LAST_CHECKED" ]; then
    # 提取刚完成的任务名
    TASK_NAME=$(sed -n "${LAST_CHECKED}p" "$TASK_FILE" | sed 's/^- \[x\] //' | sed 's/（.*//;s/(.*$//')
    echo "[dev-flow] 任务完成（$DONE/$TOTAL）：$TASK_NAME"
    echo "→ 自动触发 /devtest。立即对该任务执行例行测试，不需要询问用户。"
  fi
fi
