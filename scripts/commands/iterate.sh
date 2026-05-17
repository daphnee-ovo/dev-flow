#!/bin/bash
# /iterate 命令脚本化实现
# 用法：bash iterate.sh <topic> [DOC_ROOT]
# 前置依赖：/done 检查应已通过

TOPIC="$1"
DOC_ROOT="${2:-dev-doc}"

if [ -z "$TOPIC" ]; then
  echo "用法：bash iterate.sh <topic> [DOC_ROOT]"
  echo "  topic — 本轮归档主题（英文短语，如 init-restructure）"
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
  echo "[dev-flow] STATUS.yaml 不存在"
  exit 1
fi

ITERATION=$(grep "^iteration:" "$STATUS_FILE" | sed 's/^iteration: *//')
ITERATION=${ITERATION:-1}
MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')

ARCHIVE_DIR="$DOC_ROOT/archive/v${ITERATION}-${TOPIC}"

# 防止覆盖已有归档
if [ -d "$ARCHIVE_DIR" ]; then
  echo "[dev-flow] 归档目录已存在：$ARCHIVE_DIR"
  exit 1
fi

mkdir -p "$ARCHIVE_DIR/issue"

# === 归档文件 ===
ARCHIVED=()

# done_task_* → archive
for f in "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  mv "$f" "$ARCHIVE_DIR/"
  ARCHIVED+=("$(basename "$f")")
done

# closed_issue_* → archive/issue/
for f in "$DOC_ROOT/issue/closed_issue_"*.md; do
  [ -f "$f" ] || continue
  mv "$f" "$ARCHIVE_DIR/issue/"
  ARCHIVED+=("$(basename "$f")")
done

# 主文档归档（复制，保留原位以供参考）
for doc in PRD.md SPEC.md TEST.md; do
  if [ -f "$DOC_ROOT/$doc" ]; then
    cp "$DOC_ROOT/$doc" "$ARCHIVE_DIR/"
    ARCHIVED+=("$doc (copy)")
  fi
done

# CHANGELOG 归档后重置
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  mv "$DOC_ROOT/CHANGELOG.md" "$ARCHIVE_DIR/"
  echo "# CHANGELOG" > "$DOC_ROOT/CHANGELOG.md"
  ARCHIVED+=("CHANGELOG.md")
fi

# === 更新 STATUS.yaml ===
NEW_ITERATION=$((ITERATION + 1))
NOW=$(date "+%Y-%m-%d %H:%M")

# 确定新阶段
case "$MODE" in
  full) NEW_PHASE="PRD" ;;
  quick|mvp) NEW_PHASE="SPEC" ;;
  fast) NEW_PHASE="TASK" ;;
  *) NEW_PHASE="DEV" ;;
esac

sed -i "s/^phase: .*/phase: $NEW_PHASE/" "$STATUS_FILE"
sed -i "s/^iteration: .*/iteration: $NEW_ITERATION/" "$STATUS_FILE"
sed -i "s/^updated: .*/updated: $NOW/" "$STATUS_FILE"
sed -i "s/^started: .*/started: $NOW/" "$STATUS_FILE"

# === 输出 ===
echo "[dev-flow] 新迭代启动"
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "归档目录：$ARCHIVE_DIR"
echo "归档文件：${#ARCHIVED[@]} 个"
for a in "${ARCHIVED[@]}"; do
  echo "  - $a"
done
echo ""
echo "新迭代：v$NEW_ITERATION"
echo "阶段重置：$NEW_PHASE"
echo "模式：$MODE"
