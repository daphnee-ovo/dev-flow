#!/bin/bash
# Hook: PreToolUse(Write|Edit)
# Block source edits outside DEV.
# Allowlist: dev-doc/*, CLAUDE.md, AGENTS.md, .claude/*, tests/*, project temp dirs
# exit 2 = block tool execution, exit 0 = allow

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

DOC_ROOT=$(devflow_resolve_doc_root "dev-doc")
STATUS_FILE="$DOC_ROOT/STATUS.yaml"

if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)

if [ "$PHASE" = "DEV" ]; then
  exit 0
fi

TOOL_INPUT="${CLAUDE_TOOL_INPUT:-${CODEX_TOOL_INPUT:-}}"
if [ -z "$TOOL_INPUT" ] && [ ! -t 0 ]; then
  TOOL_INPUT="$(cat)"
fi

FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | devflow_json_field "file_path" | head -1)

if [ -z "$FILE_PATH" ]; then
  exit 0
fi

PROJECT_ROOT=$(pwd)
REL_PATH="$FILE_PATH"
if [[ "$FILE_PATH" == "$PROJECT_ROOT"/* ]]; then
  REL_PATH="${FILE_PATH#$PROJECT_ROOT/}"
fi

case "$REL_PATH" in
  dev-doc/*|CLAUDE.md|AGENTS.md|.claude/*|tests/*|t""mp/*|temp/*)
    exit 0
    ;;
esac

case "$FILE_PATH" in
  */dev-doc/*|*/CLAUDE.md|*/AGENTS.md|*/.claude/*|*/tests/*|*/t""mp/*|*/temp/*)
    exit 0
    ;;
esac

echo "BLOCKED: 当前阶段是 ${PHASE}，不允许修改项目源码。请先执行 /iterate → /task 进入 DEV 阶段后再修改代码。"
exit 2
