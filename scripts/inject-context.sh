#!/bin/bash
# Hook: UserPromptSubmit
# 每次用户发消息时，注入当前项目阶段上下文
# 让 Claude 永远知道项目在哪个阶段、该做什么

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

STATUS_FILE="$DOC_ROOT/STATUS.md"
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 读取当前阶段
PHASE=$(grep "当前阶段" "$STATUS_FILE" | sed 's/.*：//' | tr -d ' ')
if [ -z "$PHASE" ]; then
  exit 0
fi

# 读取任务进度
TOTAL=0
DONE=0
if [ -f "$DOC_ROOT/TASK.md" ]; then
  TOTAL=$(grep -c "^- \[" "$DOC_ROOT/TASK.md" 2>/dev/null || echo 0)
  DONE=$(grep -c "^- \[x\]" "$DOC_ROOT/TASK.md" 2>/dev/null || echo 0)
fi

# 统计未关闭 issue
OPEN_ISSUES=0
if [ -d "$DOC_ROOT/issue" ]; then
  OPEN_ISSUES=$(find "$DOC_ROOT/issue" -name "*.md" ! -name "*.closed.md" 2>/dev/null | wc -l)
fi

# 输出上下文注入
echo "[dev-flow] 当前阶段：$PHASE | 任务进度：$DONE/$TOTAL | 未关闭 Issue：$OPEN_ISSUES"

case "$PHASE" in
  PRD)
    echo "→ 你正在需求探索阶段。完成后用 /spec 推进。"
    ;;
  SPEC)
    echo "→ 你正在技术规范阶段。完成后用 /task 推进。"
    ;;
  TASK)
    echo "→ 你正在任务拆解阶段。完成后进入开发。"
    ;;
  DEV)
    echo "→ 你正在开发阶段。每完成一个任务必须立即：1) 勾选 TASK.md 2) 执行 /dev-test 验证。"
    if [ "$OPEN_ISSUES" -gt 0 ]; then
      echo "⚠ 有 $OPEN_ISSUES 个未关闭 issue 需要修复。"
    fi
    ;;
  TEST)
    echo "→ 你正在项目测试阶段。修复所有 issue 后进入交付。"
    if [ "$OPEN_ISSUES" -gt 0 ]; then
      echo "⚠ 有 $OPEN_ISSUES 个未关闭 issue。"
    fi
    ;;
  DONE)
    echo "→ 项目已交付。"
    ;;
esac
