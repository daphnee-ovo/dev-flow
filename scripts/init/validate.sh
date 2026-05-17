#!/bin/bash
# init 专用：校验 dev-doc 目录规范，输出结构化报告
# 用法：bash validate.sh [DOC_ROOT]
# 输出 JSON 格式报告，agent 只需处理需要确认的项

DOC_ROOT="${1:-dev-doc}"

# === 结果收集 ===
AUTO_FIXED=()
NEEDS_CONFIRM=()
WARNINGS=()

# === 1. 目录结构校验 ===
for dir in "$DOC_ROOT/issue" "$DOC_ROOT/task" "$DOC_ROOT/archive" "tests" "tmp"; do
  if [ ! -d "$dir" ]; then
    mkdir -p "$dir"
    AUTO_FIXED+=("created_dir:$dir")
  fi
done

# === 2. STATUS.yaml 校验 ===
STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ -f "$STATUS_FILE" ]; then
  # 检查必需字段
  for field in name phase mode iteration updated started; do
    if ! grep -q "^${field}:" "$STATUS_FILE"; then
      WARNINGS+=("status_missing_field:$field")
    fi
  done
  # 校验 phase 值
  PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
  if [ -n "$PHASE" ] && ! echo "$PHASE" | grep -qE '^(PRD|SPEC|TASK|DEV|TEST|DONE)$'; then
    WARNINGS+=("status_invalid_phase:$PHASE")
  fi
  # 校验 mode 值
  MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')
  if [ -n "$MODE" ] && ! echo "$MODE" | grep -qE '^(full|quick|fast|mvp)$'; then
    WARNINGS+=("status_invalid_mode:$MODE")
  fi
fi

# === 3. task/ 目录校验 ===
if [ -d "$DOC_ROOT/task" ]; then
  for f in "$DOC_ROOT/task/"task_*.md; do
    [ -f "$f" ] || continue
    BASENAME=$(basename "$f")
    # 检查命名规范
    if ! echo "$BASENAME" | grep -qE '^(done_)?task_[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]+\.md$'; then
      NEEDS_CONFIRM+=("task_bad_name:$BASENAME")
    fi
    # 检查 Done when
    TOTAL_TASKS=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL_TASKS=${TOTAL_TASKS:-0}
    TASKS_WITH_DONE_WHEN=$(grep -c "Done when" "$f" 2>/dev/null) || true; TASKS_WITH_DONE_WHEN=${TASKS_WITH_DONE_WHEN:-0}
    MISSING_DONE_WHEN=$((TOTAL_TASKS - TASKS_WITH_DONE_WHEN))
    if [ "$MISSING_DONE_WHEN" -gt 0 ]; then
      WARNINGS+=("task_missing_done_when:$BASENAME:$MISSING_DONE_WHEN")
    fi
  done
fi

# === 4. Issue 文件校验 ===
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/"*.md; do
    [ -f "$f" ] || continue
    BASENAME=$(basename "$f")
    # 检查命名规范
    if ! echo "$BASENAME" | grep -qE '^(closed_)?issue_(test|devtest|other)_[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]+\.md$'; then
      NEEDS_CONFIRM+=("issue_bad_name:$BASENAME")
    else
      # 检查 frontmatter
      if ! head -1 "$f" | grep -q '^---$'; then
        WARNINGS+=("issue_missing_frontmatter:$BASENAME")
      fi
      # 检查 checkbox/prefix 一致性
      TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL=${TOTAL:-0}
      DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; DONE=${DONE:-0}
      ALL_DONE=false
      [ "$TOTAL" -gt 0 ] && [ "$TOTAL" -eq "$DONE" ] && ALL_DONE=true
      if [ "$ALL_DONE" = true ] && [[ ! "$BASENAME" == closed_* ]]; then
        NEEDS_CONFIRM+=("issue_should_be_closed:$BASENAME")
      fi
      if [ "$ALL_DONE" = false ] && [[ "$BASENAME" == closed_* ]]; then
        NEEDS_CONFIRM+=("issue_closed_but_open_items:$BASENAME")
      fi
    fi
  done
fi

# === 5. CHANGELOG 校验 ===
CHANGELOG="$DOC_ROOT/CHANGELOG.md"
if [ -f "$CHANGELOG" ]; then
  # 检查是否有内容（非空）
  if [ ! -s "$CHANGELOG" ]; then
    WARNINGS+=("changelog_empty")
  fi
else
  # CHANGELOG 不存在时自动创建
  echo "# CHANGELOG" > "$CHANGELOG"
  AUTO_FIXED+=("created_changelog")
fi

# === 6. .gitignore 检查 ===
if [ -f ".gitignore" ]; then
  if ! grep -q "^tmp/" ".gitignore"; then
    echo "tmp/" >> ".gitignore"
    AUTO_FIXED+=("gitignore_added_tmp")
  fi
else
  echo "tmp/" > ".gitignore"
  AUTO_FIXED+=("gitignore_created")
fi

# === 输出报告 ===
echo "=== VALIDATE REPORT ==="
echo "doc_root: $DOC_ROOT"
echo ""
echo "auto_fixed:"
for item in "${AUTO_FIXED[@]}"; do
  echo "  - $item"
done
echo ""
echo "needs_confirm:"
for item in "${NEEDS_CONFIRM[@]}"; do
  echo "  - $item"
done
echo ""
echo "warnings:"
for item in "${WARNINGS[@]}"; do
  echo "  - $item"
done
echo ""
echo "summary: auto=${#AUTO_FIXED[@]} confirm=${#NEEDS_CONFIRM[@]} warn=${#WARNINGS[@]}"
