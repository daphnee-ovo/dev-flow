#!/bin/bash
# Hook: PreToolUse(Write|Edit|Bash)
# 禁止使用系统 /tmp/ 目录，临时文件只能放项目 tmp/ 下
# 只检查操作字段（file_path / command），不检查文档内容
# exit 2 = 阻断工具执行

TOOL_INPUT="${CLAUDE_TOOL_INPUT:-${CODEX_TOOL_INPUT:-}}"
if [ -z "$TOOL_INPUT" ] && [ ! -t 0 ]; then
  TOOL_INPUT="$(cat)"
fi

TOOL_NAME="${CLAUDE_TOOL_NAME:-${CODEX_TOOL_NAME:-}}"

CHECK_TEXT=""
case "$TOOL_NAME" in
  Bash)
    CHECK_TEXT=$(echo "$TOOL_INPUT" | grep -oP '"command"\s*:\s*"\K[^"]*' | head -1)
    ;;
  Write|Edit)
    CHECK_TEXT=$(echo "$TOOL_INPUT" | grep -oP '"file_path"\s*:\s*"\K[^"]*' | head -1)
    ;;
  *)
    exit 0
    ;;
esac

if echo "$CHECK_TEXT" | grep -qE '(^|[ /])/tmp(/|[ ]|$)'; then
  echo "BLOCKED: 禁止使用系统 /tmp/ 目录。临时文件请放在项目 tmp/ 下。"
  exit 2
fi

exit 0
