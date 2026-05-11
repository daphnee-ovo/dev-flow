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
  # 全部完成 → 提醒进入项目 TEST
  echo "[dev-flow] ⚠ 所有任务已完成（$DONE/$TOTAL）。"
  echo "→ 必须立即执行 /test 进行项目级全量验证。不要直接向用户报告'开发完成'。"
else
  # 有新勾选 → 提醒执行例行测试
  # 通过检查最近勾选的任务来判断是否需要提醒
  LAST_CHECKED=$(grep -n "^- \[x\]" "$TASK_FILE" | tail -1 | cut -d: -f1)
  if [ -n "$LAST_CHECKED" ]; then
    echo "[dev-flow] 检测到任务勾选（$DONE/$TOTAL 完成）。"
    echo "→ 如果刚完成了任务，必须执行 /dev-test 验证后才能继续下一个任务。"
  fi
fi
