#!/bin/bash
# Hook: PostToolUse(Write|Edit) on 代码文件
# 检测代码变更后提醒同步文档
# 只在 DEV 阶段生效

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# 确定文档根目录
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.yaml" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 只在 DEV 阶段检查
PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
if [ "$PHASE" != "DEV" ]; then
  exit 0
fi

# 获取被修改的文件（从 stdin 或环境变量）
CHANGED_FILE="${TOOL_INPUT_FILE_PATH:-$1}"
if [ -z "$CHANGED_FILE" ]; then
  exit 0
fi

# 跳过 dev-doc 目录自身的文件
if [[ "$CHANGED_FILE" == dev-doc/* ]]; then
  exit 0
fi

# 只关心代码文件
case "$CHANGED_FILE" in
  *.py|*.js|*.ts|*.tsx|*.jsx|*.rs|*.go|*.java|*.rb|*.php|*.vue|*.svelte)
    ;;
  *)
    exit 0
    ;;
esac

# fast 模式下无 SPEC，跳过
MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')
if [ "$MODE" = "fast" ]; then
  exit 0
fi

# 检查 SPEC.md 中是否有相关内容可能需要同步
SPEC_FILE="$DOC_ROOT/SPEC.md"
if [ ! -f "$SPEC_FILE" ]; then
  exit 0
fi

BASENAME=$(basename "$CHANGED_FILE")
MODULE_NAME="${BASENAME%.*}"

# 检查代码文件对应的模块是否在 SPEC 中有描述
if grep -qi "$MODULE_NAME" "$SPEC_FILE" 2>/dev/null; then
  echo "[dev-flow] 代码文件 $CHANGED_FILE 已修改，SPEC.md 中有该模块的描述。"
  echo "→ 如果修改了 API 接口/数据结构/目录组织，必须同步更新 SPEC.md。"
fi
