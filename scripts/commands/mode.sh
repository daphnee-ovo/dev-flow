#!/bin/bash
# /mode 命令脚本化实现
# 用法：bash mode.sh <full|quick|fast|mvp> [DOC_ROOT]

MODE="$1"
DOC_ROOT="${2:-dev-doc}"

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

# 检测多工程模式
if find "$DOC_ROOT" -maxdepth 2 -name "STATUS.yaml" -path "*/*/STATUS.yaml" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  if [ -n "$BRANCH" ] && [ -f "$DOC_ROOT/$BRANCH/STATUS.yaml" ]; then
    DOC_ROOT="$DOC_ROOT/$BRANCH"
  fi
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"

if [ ! -f "$STATUS_FILE" ]; then
  # STATUS 不存在则创建
  mkdir -p "$DOC_ROOT"
  NOW=$(date "+%Y-%m-%d %H:%M")
  cat > "$STATUS_FILE" << EOF
name: $(basename "$(pwd)")
phase: $(case "$MODE" in full) echo "PRD";; quick|mvp) echo "SPEC";; fast) echo "TASK";; esac)
mode: $MODE
updated: $NOW
started: $NOW
EOF
  echo "[dev-flow] 模式已设置：$MODE（新建 STATUS.yaml）"
else
  # 更新 mode 字段
  sed -i "s/^mode: .*/mode: $MODE/" "$STATUS_FILE"
  # 更新时间戳
  NOW=$(date "+%Y-%m-%d %H:%M")
  sed -i "s/^updated: .*/updated: $NOW/" "$STATUS_FILE"
  echo "[dev-flow] 模式已设置：$MODE"
fi

# 输出模式对应的阶段流程
case "$MODE" in
  full) echo "流程：PRD → SPEC → TASK → DEV → TEST → ITERATE" ;;
  quick) echo "流程：SPEC → TASK → DEV → TEST → ITERATE" ;;
  fast) echo "流程：TASK → DEV → TEST → ITERATE" ;;
  mvp) echo "流程：SPEC → TASK → DEV → ITERATE" ;;
esac
