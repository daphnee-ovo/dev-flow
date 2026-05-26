#!/bin/bash
# /mode 命令脚本化实现
# 用法：bash mode.sh <full|quick|fast|mvp> [DOC_ROOT]

MODE="$1"
DOC_ROOT="${2:-dev-doc}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

if [ -z "$MODE" ]; then
  echo "用法：bash mode.sh <full|quick|fast|mvp>"
  echo ""
  echo "模式说明："
  echo "  full  — PRD → SPEC → TASK → DEV → TEST → ITERATE"
  echo "  quick — SPEC → TASK → DEV → TEST → ITERATE"
  echo "  fast  — TASK → DEV → TEST → ITERATE"
  echo "  mvp   — SPEC → TASK → DEV → ITERATE"
  exit 1
fi

# 校验合法性
if ! echo "$MODE" | grep -qE '^(full|quick|fast|mvp)$'; then
  echo "[dev-flow] 无效模式：$MODE（可选：full/quick/fast/mvp）"
  exit 1
fi

DOC_ROOT=$(devflow_resolve_doc_root "$DOC_ROOT")

STATUS_FILE="$DOC_ROOT/STATUS.yaml"

if [ ! -f "$STATUS_FILE" ]; then
  # STATUS 不存在则创建
  mkdir -p "$DOC_ROOT"
  NOW=$(date "+%Y-%m-%d %H:%M")
  case "$MODE" in
    full) INIT_PHASE="PRD" ;;
    quick|mvp) INIT_PHASE="SPEC" ;;
    fast) INIT_PHASE="TASK" ;;
  esac
  cat > "$STATUS_FILE" << EOF
name: $(basename "$(pwd)")
phase: $INIT_PHASE
mode: $MODE
updated: $NOW
started: $NOW
EOF
  echo "[dev-flow] 模式已设置：${MODE}（新建 STATUS.yaml）"
else
  # 更新 mode 字段
  devflow_yaml_set "$STATUS_FILE" mode "$MODE"
  # 更新时间戳
  NOW=$(date "+%Y-%m-%d %H:%M")
  devflow_yaml_set "$STATUS_FILE" updated "$NOW"
  echo "[dev-flow] 模式已设置：$MODE"
fi

# 输出模式对应的阶段流程
case "$MODE" in
  full) echo "流程：PRD → SPEC → TASK → DEV → TEST → ITERATE" ;;
  quick) echo "流程：SPEC → TASK → DEV → TEST → ITERATE" ;;
  fast) echo "流程：TASK → DEV → TEST → ITERATE" ;;
  mvp) echo "流程：SPEC → TASK → DEV → ITERATE" ;;
esac
