#!/bin/bash
# Hook: PreToolUse(Write|Edit|Bash)
# 禁止使用系统 /tmp/ 目录，临时文件只能放项目 tmp/ 下
# exit 2 = 阻断工具执行

TOOL_INPUT="${CLAUDE_TOOL_INPUT:-${CODEX_TOOL_INPUT:-}}"
if [ -z "$TOOL_INPUT" ] && [ ! -t 0 ]; then
  TOOL_INPUT="$(cat)"
fi

if echo "$TOOL_INPUT" | grep -qE '(^|[" /])/tmp(/|[" ])'; then
  echo "BLOCKED: 禁止使用系统 /tmp/ 目录。临时文件请放在项目 tmp/ 下。"
  exit 2
fi

exit 0
