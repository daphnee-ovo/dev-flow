#!/bin/bash
# /iterate 命令：交付检查 → 归档 → commit & tag → bump 版本
# 用法：bash iterate.sh <topic> [bump_type] [DOC_ROOT]
# bump_type: minor（默认）| major | patch

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/version.sh"

TOPIC="$1"
BUMP_TYPE="${2:-minor}"
DOC_ROOT="${3:-dev-doc}"

if [ -z "$TOPIC" ]; then
  echo "用法：bash iterate.sh <topic> [bump_type] [DOC_ROOT]"
  echo "  topic     — 本轮归档主题"
  echo "  bump_type — major|minor|patch（默认 minor）"
  exit 1
fi

# 检测多工程模式（分支 ↔ 文档路径一致性）
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "$DOC_ROOT/$BRANCH/STATUS.yaml" ]; then
  DOC_ROOT="$DOC_ROOT/$BRANCH"
fi

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  echo "[dev-flow] ERROR: STATUS.yaml 不存在：$STATUS_FILE"
  exit 1
fi

MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')

# ===== 阶段 1：交付检查（阻断） =====
TASK_TOTAL=0; TASK_DONE=0
for f in "$DOC_ROOT/task/task_"*.md "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TASK_TOTAL=$((TASK_TOTAL + ${CNT:-0}))
  CNT=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; TASK_DONE=$((TASK_DONE + ${CNT:-0}))
done

if [ "$TASK_TOTAL" -gt 0 ] && [ "$TASK_DONE" -lt "$TASK_TOTAL" ]; then
  echo "[dev-flow] ERROR: 任务未全部完成（$TASK_DONE/$TASK_TOTAL）"
  echo "→ 请完成所有任务后再执行 /iterate"
  exit 1
fi

# 检查未关闭 P0 issue（只计算 [ ] 状态的 P0 条目）
P0_OPEN=0
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    # 逐条检查：只有 [ ]（未关闭）的条目后跟 severity: P0 才算
    while IFS= read -r line; do
      if echo "$line" | grep -qE "^- \[ \]"; then
        CURRENT_OPEN=true
      elif echo "$line" | grep -qE "^- \[x\]"; then
        CURRENT_OPEN=false
      elif [ "$CURRENT_OPEN" = true ] && echo "$line" | grep -q "severity: P0"; then
        P0_OPEN=$((P0_OPEN + 1))
        CURRENT_OPEN=false
      fi
    done < "$f"
  done
fi

if [ "$P0_OPEN" -gt 0 ]; then
  echo "[dev-flow] ERROR: 有 $P0_OPEN 个未关闭的 P0 issue"
  echo "→ 请修复所有 P0 issue 后再执行 /iterate"
  exit 1
fi

# ===== 阶段 2：读取当前版本 =====
VERSION=$(version_read)
if [ -z "$VERSION" ]; then
  echo "[dev-flow] ERROR: VERSION 文件不存在或为空"
  exit 1
fi
if ! version_validate "$VERSION"; then
  echo "[dev-flow] ERROR: VERSION 文件格式非法: $VERSION"
  exit 1
fi

NEW_VERSION=$(version_bump "$VERSION" "$BUMP_TYPE")

# ===== 阶段 3：归档（用当前版本号命名） =====
ARCHIVE_DIR="$DOC_ROOT/archive/v${VERSION}-${TOPIC}"

if [ -d "$ARCHIVE_DIR" ]; then
  echo "[dev-flow] ERROR: 归档目录已存在：$ARCHIVE_DIR"
  exit 1
fi

mkdir -p "$ARCHIVE_DIR/issue"

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

# 主文档归档（复制）
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

# ===== 阶段 4：展示变更摘要 =====
echo "[dev-flow] 迭代摘要"
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "当前版本：v$VERSION"
echo "归档目录：$ARCHIVE_DIR"
echo "归档文件：${#ARCHIVED[@]} 个"
for a in "${ARCHIVED[@]}"; do
  echo "  - $a"
done
echo ""
echo "将要执行："
echo "  - git commit: \"Release v${VERSION}: ${TOPIC}\""
echo "  - git tag: v${VERSION}"
echo "  - bump 版本：v${VERSION} → v${NEW_VERSION}"
echo ""

# 非交互模式（由 agent 调用时跳过确认）
if [ "${DEVFLOW_NO_CONFIRM:-}" != "1" ]; then
  echo "等待 agent 确认后继续..."
  exit 0
fi

# ===== 阶段 5：commit & tag =====
git add -A
git commit -m "Release v${VERSION}: ${TOPIC}"

if version_tag_exists "$VERSION"; then
  echo "[dev-flow] WARNING: tag v$VERSION 已存在，跳过创建"
else
  version_create_tag "$VERSION"
  echo "[dev-flow] 已创建 git tag: v$VERSION"
fi

# ===== 阶段 6：bump 版本号，开启新迭代 =====
version_write "$NEW_VERSION"

# 重置 STATUS.yaml phase
case "$MODE" in
  full) NEW_PHASE="PRD" ;;
  quick|mvp) NEW_PHASE="SPEC" ;;
  fast) NEW_PHASE="TASK" ;;
  *) NEW_PHASE="DEV" ;;
esac

NOW=$(date "+%Y-%m-%d %H:%M")
sed -i "s/^phase: .*/phase: $NEW_PHASE/" "$STATUS_FILE"
sed -i "s/^updated: .*/updated: $NOW/" "$STATUS_FILE"
sed -i "s/^started: .*/started: $NOW/" "$STATUS_FILE"

git add VERSION "$STATUS_FILE"
git commit -m "Start v${NEW_VERSION} iteration"

# ===== 输出 =====
echo ""
echo "[dev-flow] 迭代完成"
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "交付版本：v$VERSION (tagged)"
echo "新版本：v$NEW_VERSION"
echo "阶段重置：$NEW_PHASE"
echo "模式：$MODE"
