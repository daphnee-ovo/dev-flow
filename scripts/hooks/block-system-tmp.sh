#!/bin/bash
# Hook: PreToolUse(Write|Edit|Bash)
# Block writes to the system scratch directory while allowing harmless mentions.
# exit 2 = block tool execution

TOOL_INPUT="${CLAUDE_TOOL_INPUT:-${CODEX_TOOL_INPUT:-}}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"
if [ -z "$TOOL_INPUT" ] && [ ! -t 0 ]; then
  TOOL_INPUT="$(cat)"
fi

TOOL_NAME="${CLAUDE_TOOL_NAME:-${CODEX_TOOL_NAME:-}}"
SYSTEM_SCRATCH="$(printf '\057\164\155\160')"
PROJECT_TEMP_HINT="tmp or temp"
MAKE_TEMP_CMD="$(printf 'mk%s' 'temp')"

contains_system_scratch_path() {
  local text="$1"
  printf '%s\n' "$text" | grep -qE "(^|[[:space:]\"'=(:])${SYSTEM_SCRATCH}(/|[[:space:]\"')]|$)"
}

is_write_like_scratch_command() {
  local command="$1"

  contains_system_scratch_path "$command" || return 1

  if [[ "$command" == *"$MAKE_TEMP_CMD"* ]]; then
    return 0
  fi

  case "$command" in
    *">"*|*">>"*|*"tee "*|*"mkdir "*|*"touch "*|*"cp "*|*"mv "*|*"install "*|*"rsync "*|*"tar "*|*"unzip "*|*"python "*|*"python3 "*|*"node "*|*"perl "*|*"ruby "*|*"sh "*|*"bash "*)
      return 0
      ;;
  esac

  return 1
}

CHECK_TEXT=""
case "$TOOL_NAME" in
  Bash)
    CHECK_TEXT=$(printf '%s\n' "$TOOL_INPUT" | devflow_json_field "command" | head -1)
    if is_write_like_scratch_command "$CHECK_TEXT"; then
      echo "BLOCKED: 禁止向系统临时目录写入文件。临时文件请放在项目 ${PROJECT_TEMP_HINT} 下。"
      exit 2
    fi
    ;;
  Write|Edit)
    CHECK_TEXT=$(printf '%s\n' "$TOOL_INPUT" | devflow_json_field "file_path" | head -1)
    if contains_system_scratch_path "$CHECK_TEXT"; then
      echo "BLOCKED: 禁止向系统临时目录写入文件。临时文件请放在项目 ${PROJECT_TEMP_HINT} 下。"
      exit 2
    fi
    ;;
  *)
    exit 0
    ;;
esac

exit 0
