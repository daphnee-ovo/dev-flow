#!/bin/bash
# init 专用：检测旧格式并迁移到新结构
# 用法：bash migrate.sh [DOC_ROOT]
# 输出迁移报告，仅在检测到旧格式时执行

DOC_ROOT="${1:-dev-doc}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"
TODAY=$(date +%Y-%m-%d)
MIGRATED=()
SKIPPED=()

# === 1. TASK.md → task/ 目录 ===
if [ -f "$DOC_ROOT/TASK.md" ]; then
  mkdir -p "$DOC_ROOT/task"
  TARGET="$DOC_ROOT/task/task_${TODAY}_1.md"
  # 避免覆盖已有文件
  SEQ=1
  while [ -f "$TARGET" ]; do
    SEQ=$((SEQ + 1))
    TARGET="$DOC_ROOT/task/task_${TODAY}_${SEQ}.md"
  done
  cp "$DOC_ROOT/TASK.md" "$TARGET"
  mv "$DOC_ROOT/TASK.md" "$DOC_ROOT/TASK.md.bak"
  MIGRATED+=("TASK.md → $TARGET (backup: TASK.md.bak)")
fi

# === 2. session/ → CHANGELOG.md ===
if [ -d "$DOC_ROOT/session" ]; then
  SESSION_FILES=$(find "$DOC_ROOT/session" -maxdepth 1 -name "*.md" 2>/dev/null | sort)
  if [ -n "$SESSION_FILES" ]; then
    # 生成 CHANGELOG（如果不存在）
    if [ ! -f "$DOC_ROOT/CHANGELOG.md" ]; then
      echo "# CHANGELOG" > "$DOC_ROOT/CHANGELOG.md"
      echo "" >> "$DOC_ROOT/CHANGELOG.md"
      echo "## $TODAY (migrated from session/)" >> "$DOC_ROOT/CHANGELOG.md"
      echo "" >> "$DOC_ROOT/CHANGELOG.md"
      # 从 session 文件提取摘要
      while IFS= read -r f; do
        [ -f "$f" ] || continue
        BASENAME=$(basename "$f" .md)
        # 提取 topic 部分（去掉序号前缀）
        TOPIC=$(echo "$BASENAME" | sed 's/^[0-9]*-//')
        echo "- 00:00 $TOPIC: (migrated from session)" >> "$DOC_ROOT/CHANGELOG.md"
      done <<< "$SESSION_FILES"
      MIGRATED+=("session/*.md → CHANGELOG.md (${#SESSION_FILES} sessions)")
    else
      SKIPPED+=("session/ exists but CHANGELOG.md already present, skipping")
    fi
    # 保留 session/ 目录（不删除，让用户自行处理）
  fi
fi

# === 3. STATUS.yaml phase=MVP → phase=DEV ===
STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ -f "$STATUS_FILE" ]; then
  PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)
  if [ "$PHASE" = "MVP" ]; then
    devflow_yaml_set "$STATUS_FILE" phase "DEV"
    MIGRATED+=("STATUS.yaml phase: MVP → DEV")
  fi
fi

# === 输出报告 ===
echo "=== MIGRATION REPORT ==="

if [ ${#MIGRATED[@]} -eq 0 ] && [ ${#SKIPPED[@]} -eq 0 ]; then
  echo "status: no_migration_needed"
else
  echo "status: migration_performed"
  echo ""
  echo "migrated:"
  for item in "${MIGRATED[@]}"; do
    echo "  - $item"
  done
  echo ""
  echo "skipped:"
  for item in "${SKIPPED[@]}"; do
    echo "  - $item"
  done
fi
