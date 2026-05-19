#!/bin/bash
# Hook: PreToolUse(Write|Edit)
# 阻止在非 DEV 阶段修改项目源码
# 白名单：dev-doc/*, CLAUDE.md, AGENTS.md, .claude/*, tests/*, tmp/*
# exit 2 = 阻断工具执行, exit 0 = 放行

STATUS_FILE="dev-doc/STATUS.yaml"

# 如果 STATUS.yaml 不存在，放行（项目未初始化）
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 读取当前 phase
PHASE=$(grep -oP '^phase:\s*\K\S+' "$STATUS_FILE" 2>/dev/null)

# 如果是 DEV 阶段，放行
if [ "$PHASE" = "DEV" ]; then
  exit 0
fi

# 非 DEV 阶段，检查写入路径
TOOL_INPUT="${CLAUDE_TOOL_INPUT:-${CODEX_TOOL_INPUT:-}}"
if [ -z "$TOOL_INPUT" ] && [ ! -t 0 ]; then
  TOOL_INPUT="$(cat)"
fi

FILE_PATH=$(echo "$TOOL_INPUT" | grep -oP '"file_path"\s*:\s*"\K[^"]*' | head -1)

# 如果无法提取路径，放行
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# 白名单检查（支持绝对路径和相对路径）
# 去掉可能的项目根目录前缀，得到相对路径
PROJECT_ROOT=$(pwd)
REL_PATH="$FILE_PATH"
if [[ "$FILE_PATH" == "$PROJECT_ROOT"/* ]]; then
  REL_PATH="${FILE_PATH#$PROJECT_ROOT/}"
fi

# 白名单匹配
case "$REL_PATH" in
  dev-doc/*|CLAUDE.md|AGENTS.md|.claude/*|tests/*|tmp/*)
    exit 0
    ;;
esac

# 绝对路径白名单匹配
case "$FILE_PATH" in
  */dev-doc/*|*/CLAUDE.md|*/AGENTS.md|*/.claude/*|*/tests/*|*/tmp/*)
    exit 0
    ;;
esac

# 阻断
echo "BLOCKED: 当前阶段是 ${PHASE}，不允许修改项目源码。请先执行 /iterate → /task 进入 DEV 阶段后再修改代码。"
exit 2
