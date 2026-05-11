#!/bin/bash
# Hook: session 结束时自动保存会话记录
# 触发时机：Stop
# 支持单工程和多工程模式

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

mkdir -p "$DOC_ROOT/session/task"

DATE=$(date +%Y-%m-%d)
TIME=$(date +%H%M)
SESSION_FILE="$DOC_ROOT/session/task/task-session-${DATE}-${TIME}.md"

# 当天已有记录则不重复创建
if ls "$DOC_ROOT"/session/task/task-session-${DATE}*.md 1>/dev/null 2>&1; then
  exit 0
fi

# 读取当前阶段
PHASE="unknown"
if [ -f "$DOC_ROOT/STATUS.md" ]; then
  PHASE=$(grep "当前阶段" "$DOC_ROOT/STATUS.md" | sed 's/.*：//' | tr -d ' ')
fi

cat > "$SESSION_FILE" << EOF
# 会话记录 — $DATE

**当前阶段**：$PHASE

## 本次完成
- [ ] （请补充本次完成的工作）

## 遇到的问题
-

## 下次继续
- [ ]

## 备注

EOF

echo "[dev-flow] 已创建会话记录：$SESSION_FILE"
