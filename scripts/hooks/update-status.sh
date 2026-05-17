#!/bin/bash
# Hook: dev-doc/ 下的文档变更时，自动更新 STATUS.yaml 的时间戳
# 触发时机：PostToolUse(Write|Edit)
# 支持单工程和多工程模式

if [ ! -d "dev-doc" ]; then
  exit 0
fi

CHANGED_FILE="$1"

if [[ ! "$CHANGED_FILE" == dev-doc/* ]]; then
  exit 0
fi

# 确定 STATUS.yaml 所在目录
REL_PATH="${CHANGED_FILE#dev-doc/}"
FIRST_SEGMENT="${REL_PATH%%/*}"
if [ -f "dev-doc/$FIRST_SEGMENT/STATUS.yaml" ]; then
  DOC_ROOT="dev-doc/$FIRST_SEGMENT"
else
  DOC_ROOT="dev-doc"
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"

# 跳过 STATUS.yaml 自身（避免循环）
if [[ "$CHANGED_FILE" == "$STATUS_FILE" ]]; then
  exit 0
fi

# 跳过 CHANGELOG.md（避免 save-changelog 触发循环）
FILE_IN_PROJECT="${REL_PATH#$FIRST_SEGMENT/}"
if [[ "$FILE_IN_PROJECT" == "CHANGELOG.md" ]]; then
  exit 0
fi

DATE=$(date "+%Y-%m-%d %H:%M")

if [ -f "$STATUS_FILE" ]; then
  sed -i "s/^updated:.*/updated: $DATE/" "$STATUS_FILE"
fi
