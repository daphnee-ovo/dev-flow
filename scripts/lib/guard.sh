#!/bin/bash
# 独立守卫脚本 — 提供与 post-write hook 等效的检查能力
# 用途：非 Claude Code 环境（Codex、独立 agent、CI）可直接调用
# 用法：source guard.sh && guard_check_all [DOC_ROOT]

guard_check_deps() {
  # 检查依赖违规：已完成任务的 depends on 是否满足
  local DOC_ROOT="${1:-dev-doc}"
  local DEP_VIOLATIONS=""

  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    local CURRENT_TASK="" CURRENT_DONE=false CURRENT_DEPS=""
    while IFS= read -r line; do
      if echo "$line" | grep -qE '^- \[[ x]\]'; then
        if [ "$CURRENT_DONE" = true ] && [ -n "$CURRENT_DEPS" ] && [ "$CURRENT_DEPS" != "无" ]; then
          for dep in $(echo "$CURRENT_DEPS" | sed 's/,/ /g; s/、/ /g'); do
            dep=$(echo "$dep" | tr -d '[:space:]')
            [ -z "$dep" ] && continue
            if ! grep -qE "^- \[x\] ${dep}[：:：]" "$f" 2>/dev/null; then
              DEP_VIOLATIONS="$DEP_VIOLATIONS\n  - $CURRENT_TASK depends on $dep（未完成）"
            fi
          done
        fi
        CURRENT_TASK=$(echo "$line" | sed 's/^- \[.\] //' | sed 's/[：:].*//')
        echo "$line" | grep -q "^- \[x\]" && CURRENT_DONE=true || CURRENT_DONE=false
        CURRENT_DEPS=""
      elif echo "$line" | grep -qE "^\s+- depends on"; then
        CURRENT_DEPS=$(echo "$line" | sed 's/.*depends on[：:：] *//')
      fi
    done < "$f"
    # 检查最后一个条目
    if [ "$CURRENT_DONE" = true ] && [ -n "$CURRENT_DEPS" ] && [ "$CURRENT_DEPS" != "无" ]; then
      for dep in $(echo "$CURRENT_DEPS" | sed 's/,/ /g; s/、/ /g'); do
        dep=$(echo "$dep" | tr -d '[:space:]')
        [ -z "$dep" ] && continue
        if ! grep -qE "^- \[x\] ${dep}[：:：]" "$f" 2>/dev/null; then
          DEP_VIOLATIONS="$DEP_VIOLATIONS\n  - $CURRENT_TASK depends on $dep（未完成）"
        fi
      done
    fi
  done

  if [ -n "$DEP_VIOLATIONS" ]; then
    echo "[guard] ⚠ 依赖违规：以下任务的前置依赖尚未完成："
    echo -e "$DEP_VIOLATIONS"
    return 1
  fi
  return 0
}

guard_check_batch() {
  # 检查批量完成：对比工作区中 task 文件的未提交变化
  local DOC_ROOT="${1:-dev-doc}"
  local WARNINGS=0

  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    if command -v git &>/dev/null && git rev-parse --git-dir &>/dev/null 2>&1; then
      local NEWLY_DONE=$(git diff -- "$f" 2>/dev/null | grep -c "^+- \[x\]" || true)
      if [ "${NEWLY_DONE:-0}" -gt 2 ]; then
        echo "[guard] ⚠ $f：单次变更标记了 $NEWLY_DONE 个任务完成"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
  done

  [ "$WARNINGS" -gt 0 ] && return 1
  return 0
}

guard_check_phase() {
  # 检查阶段一致性：STATUS 阶段与实际文档是否匹配
  local DOC_ROOT="${1:-dev-doc}"
  local STATUS_FILE="$DOC_ROOT/STATUS.yaml"

  [ ! -f "$STATUS_FILE" ] && echo "[guard] STATUS.yaml 不存在" && return 1

  local PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
  local ISSUES=0

  case "$PHASE" in
    DEV|TEST|DONE)
      local TOTAL=0
      for f in "$DOC_ROOT/task/task_"*.md "$DOC_ROOT/task/done_task_"*.md; do
        [ -f "$f" ] || continue
        local CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true
        TOTAL=$((TOTAL + ${CNT:-0}))
      done
      if [ "$TOTAL" -eq 0 ]; then
        echo "[guard] ⚠ 阶段为 $PHASE 但无任务文件"
        ISSUES=$((ISSUES + 1))
      fi
      ;;
  esac

  case "$PHASE" in
    SPEC|TASK|DEV|TEST|DONE)
      local MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')
      if [ "$MODE" != "fast" ] && [ ! -f "$DOC_ROOT/SPEC.md" ]; then
        echo "[guard] ⚠ 阶段为 $PHASE 但缺少 SPEC.md"
        ISSUES=$((ISSUES + 1))
      fi
      ;;
  esac

  [ "$ISSUES" -gt 0 ] && return 1
  return 0
}

guard_check_all() {
  # 执行全部检查
  local DOC_ROOT="${1:-dev-doc}"
  local FAILURES=0

  echo "[guard] 执行 dev-flow 完整性检查..."
  echo ""

  guard_check_deps "$DOC_ROOT" || FAILURES=$((FAILURES + 1))
  guard_check_batch "$DOC_ROOT" || FAILURES=$((FAILURES + 1))
  guard_check_phase "$DOC_ROOT" || FAILURES=$((FAILURES + 1))

  echo ""
  if [ "$FAILURES" -eq 0 ]; then
    echo "[guard] ✓ 全部检查通过"
  else
    echo "[guard] ✗ $FAILURES 项检查未通过"
  fi
  return $FAILURES
}
