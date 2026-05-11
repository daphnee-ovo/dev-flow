#!/bin/bash
# Hook: dev-doc/ 下的文档变更时，自动更新 STATUS.md 的最近动态
# 触发时机：PostToolUse(Write|Edit)
# 支持单工程和多工程模式

if [ ! -d "dev-doc" ]; then
  exit 0
fi

CHANGED_FILE="$1"

if [[ ! "$CHANGED_FILE" == dev-doc/* ]]; then
  exit 0
fi

# 确定 STATUS.md 所在目录
REL_PATH="${CHANGED_FILE#dev-doc/}"
FIRST_SEGMENT="${REL_PATH%%/*}"
if [ -f "dev-doc/$FIRST_SEGMENT/STATUS.md" ]; then
  DOC_ROOT="dev-doc/$FIRST_SEGMENT"
  FILE_IN_PROJECT="${REL_PATH#$FIRST_SEGMENT/}"
else
  DOC_ROOT="dev-doc"
  FILE_IN_PROJECT="$REL_PATH"
fi

STATUS_FILE="$DOC_ROOT/STATUS.md"

# 跳过 STATUS.md 自身（避免循环）
if [[ "$CHANGED_FILE" == "$STATUS_FILE" ]]; then
  exit 0
fi

# 跳过 session 目录
if [[ "$FILE_IN_PROJECT" == session/* ]]; then
  exit 0
fi

DATE=$(date +%Y-%m-%d)
BASENAME=$(basename "$CHANGED_FILE")

if [ -f "$STATUS_FILE" ]; then
  sed -i "s/^- 更新时间：.*/- 更新时间：$DATE/" "$STATUS_FILE"
  if grep -q "## 最近动态" "$STATUS_FILE"; then
    sed -i "/## 最近动态/a - $DATE: 更新 $BASENAME" "$STATUS_FILE"
  fi
fi
