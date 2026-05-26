#!/bin/bash
# 聚合 Hook: PostToolUse(Write|Edit)
# 合并 4 个独立 hook 为一次进程调用，减少串行开销
# 子逻辑：update-status / check-task-completion / check-doc-sync / check-phase-completion

if [ ! -d "dev-doc" ]; then
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

CHANGED_FILE="${TOOL_INPUT_FILE_PATH:-$1}"

# === 公共变量（只计算一次） ===
DOC_ROOT=$(devflow_resolve_doc_root "dev-doc")
STATUS_FILE="$DOC_ROOT/STATUS.yaml"

# === 1. update-status：更新时间戳 ===
if [[ "$CHANGED_FILE" == dev-doc/* ]] && [ -f "$STATUS_FILE" ]; then
  # 跳过 STATUS.yaml 自身和 CHANGELOG.md
  REL_PATH="${CHANGED_FILE#dev-doc/}"
  FIRST_SEGMENT="${REL_PATH%%/*}"
  if [ -f "dev-doc/$FIRST_SEGMENT/STATUS.yaml" ]; then
    FILE_IN_PROJECT="${REL_PATH#$FIRST_SEGMENT/}"
  else
    FILE_IN_PROJECT="$REL_PATH"
  fi

  if [[ "$CHANGED_FILE" != "$STATUS_FILE" ]] && [[ "$FILE_IN_PROJECT" != "CHANGELOG.md" ]]; then
    DATE=$(date "+%Y-%m-%d %H:%M")
    devflow_yaml_set "$STATUS_FILE" updated "$DATE"
  fi
fi

# === 以下逻辑需要 STATUS_FILE 存在 ===
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)
MODE=$(devflow_yaml_get "$STATUS_FILE" mode)

# === 1.5 audit 模式自动触发：非 DEV 阶段创建 issue 文件 ===
# 条件：文件匹配 issue_*.md + 当前非 audit 模式 + 当前非 DEV 阶段
if [[ "$CHANGED_FILE" == */issue/issue_*.md ]] && ! is_audit_mode "$MODE" && [ "$PHASE" != "DEV" ]; then
  enter_audit_mode "$STATUS_FILE"
fi

# === 2. check-task-completion：任务完成度检测（仅 DEV 阶段） ===
if [ "$PHASE" = "DEV" ]; then
  TOTAL=0; DONE=0
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL=$((TOTAL + ${CNT:-0}))
    CNT=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; DONE=$((DONE + ${CNT:-0}))
  done

  if [ "$TOTAL" -gt 0 ]; then
    # === 依赖检查：已完成任务的 depends on 是否满足 ===
    DEP_VIOLATIONS=""
    for f in "$DOC_ROOT/task/task_"*.md; do
      [ -f "$f" ] || continue
      CURRENT_TASK="" ; CURRENT_DONE=false ; CURRENT_DEPS=""
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
      echo "[dev-flow] ⚠ 依赖违规：以下任务的前置依赖尚未完成："
      echo -e "$DEP_VIOLATIONS"
      echo "→ 请先完成依赖任务，再标记当前任务为完成。"
    fi

    UNDONE=$((TOTAL - DONE))
    if [ "$UNDONE" -eq 0 ]; then
      echo "[dev-flow] 所有任务已完成（${DONE}/${TOTAL}）。"
      echo "→ 立即执行 /test 进行全量验证。"
    else
      EXEC_MODE=$(grep "^exec_mode:" "$STATUS_FILE" 2>/dev/null | sed 's/^exec_mode: *//')
      for f in "$DOC_ROOT/task/task_"*.md; do
        [ -f "$f" ] || continue
        LAST_CHECKED=$(grep -n "^- \[x\]" "$f" | tail -1 | cut -d: -f1)
        if [ -n "$LAST_CHECKED" ]; then
          TASK_NAME=$(sed -n "${LAST_CHECKED}p" "$f" | sed 's/^- \[x\] //' | sed 's/（.*//;s/(.*$//')
          if [ -n "$TASK_NAME" ]; then
            echo "[dev-flow] 任务完成（${DONE}/${TOTAL}）：$TASK_NAME"
            if [ "$EXEC_MODE" = "continuous" ]; then
              echo "→ [continuous] 自动推进：执行 /devtest 并在通过后继续下一个任务。"
            else
              echo "→ 自动触发 /devtest。立即对该任务执行例行测试，不需要询问用户。"
            fi
            break
          fi
        fi
      done
    fi

    # === 批量完成检测（I4）：单次标记过多任务完成时警告 ===
    if [[ "$CHANGED_FILE" == *task/task_*.md ]]; then
      if command -v git &>/dev/null && git rev-parse --git-dir &>/dev/null 2>&1; then
        NEWLY_DONE=$(git diff -- "$CHANGED_FILE" 2>/dev/null | grep -c "^+- \[x\]" || true)
        if [ "${NEWLY_DONE:-0}" -gt 2 ]; then
          echo "[dev-flow] ⚠ 单次写入标记了 $NEWLY_DONE 个任务完成，请确认是否逐步验证了每个任务。"
        fi
      fi
    fi

    # done_ 前缀自动重命名
    for f in "$DOC_ROOT/task/task_"*.md; do
      [ -f "$f" ] || continue
      FILE_TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; FILE_TOTAL=${FILE_TOTAL:-0}
      FILE_DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; FILE_DONE=${FILE_DONE:-0}
      if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
        BASENAME=$(basename "$f")
        NEWNAME="$DOC_ROOT/task/done_$BASENAME"
        if [ ! -f "$NEWNAME" ]; then
          mv "$f" "$NEWNAME"
          echo "[dev-flow] 批次全部完成，已标记：done_$BASENAME"
        fi
      fi
    done

    # closed_ 前缀自动重命名
    if [ -d "$DOC_ROOT/issue" ]; then
      for f in "$DOC_ROOT/issue/issue_"*.md; do
        [ -f "$f" ] || continue
        FILE_TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; FILE_TOTAL=${FILE_TOTAL:-0}
        FILE_DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; FILE_DONE=${FILE_DONE:-0}
        if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
          BASENAME=$(basename "$f")
          NEWNAME="$DOC_ROOT/issue/closed_$BASENAME"
          if [ ! -f "$NEWNAME" ]; then
            mv "$f" "$NEWNAME"
            echo "[dev-flow] Issue 全部关闭：closed_$BASENAME"
          fi
        fi
      done
    fi
  fi
fi

# === 3. check-doc-sync：代码变更提醒同步文档（仅 DEV 阶段） ===
if [ "$PHASE" = "DEV" ] && [ -n "$CHANGED_FILE" ]; then
  if [[ "$CHANGED_FILE" != dev-doc/* ]]; then
    case "$CHANGED_FILE" in
      *.py|*.js|*.ts|*.tsx|*.jsx|*.rs|*.go|*.java|*.rb|*.php|*.vue|*.svelte)
        MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')
        if [ "$MODE" != "fast" ]; then
          SPEC_FILE="$DOC_ROOT/SPEC.md"
          if [ -f "$SPEC_FILE" ]; then
            BASENAME=$(basename "$CHANGED_FILE")
            MODULE_NAME="${BASENAME%.*}"
            if grep -qi "$MODULE_NAME" "$SPEC_FILE" 2>/dev/null; then
              echo "[dev-flow] 代码文件 $CHANGED_FILE 已修改，SPEC.md 中有该模块的描述。"
              echo "→ 如果修改了 API 接口/数据结构/目录组织，必须同步更新 SPEC.md。"
            fi
          fi
        fi
        ;;
    esac
  fi
fi

# === 4. check-phase-completion：阶段文档完成标准检查 ===
if [[ "$CHANGED_FILE" == dev-doc/* ]]; then
  REL_PATH="${CHANGED_FILE#dev-doc/}"
  FIRST_SEGMENT="${REL_PATH%%/*}"
  if [ -f "dev-doc/$FIRST_SEGMENT/STATUS.yaml" ]; then
    PC_DOC_ROOT="dev-doc/$FIRST_SEGMENT"
    TARGET_FILE="${REL_PATH#$FIRST_SEGMENT/}"
  else
    PC_DOC_ROOT="dev-doc"
    TARGET_FILE="$REL_PATH"
  fi

  ISSUES=""
  case "$TARGET_FILE" in
    PRD.md)
      FILE_PATH="$PC_DOC_ROOT/PRD.md"
      if ! grep -q "## 2. 目标与非目标" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- PRD 缺少「目标与非目标」章节"
      fi
      if ! grep -q "### 非目标" "$FILE_PATH" && ! grep -q "### Won't Have" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- PRD 缺少「非目标」定义"
      fi
      if ! grep -q "## 6. 成功指标" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- PRD 缺少「成功指标」"
      fi
      if ! grep -q "Must Have" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- PRD 功能需求未分优先级"
      fi
      ;;
    SPEC.md)
      FILE_PATH="$PC_DOC_ROOT/SPEC.md"
      if ! grep -q "## 2. 架构设计" "$FILE_PATH" && ! grep -q "## 架构设计" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- SPEC 缺少「架构设计」章节"
      fi
      if ! grep -q "## 3. 技术选型" "$FILE_PATH" && ! grep -q "## 技术选型" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- SPEC 缺少「技术选型」章节"
      fi
      if ! grep -q "理由" "$FILE_PATH" && ! grep -q "原因" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- SPEC 技术选型可能缺少理由说明"
      fi
      if ! grep -q "## 4. 数据模型" "$FILE_PATH" && ! grep -q "## 数据模型" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- SPEC 缺少「数据模型」章节"
      fi
      ;;
    TASK.md)
      FILE_PATH="$PC_DOC_ROOT/TASK.md"
      if ! grep -q "Done when" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- TASK 缺少 Done when 验收标准"
      fi
      if grep -q "Done when：完成" "$FILE_PATH" || grep -q "Done when：实现" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- TASK 存在模糊的 Done when（'完成'或'实现'不是有效标准）"
      fi
      ;;
    task/task_*.md)
      FILE_PATH="$PC_DOC_ROOT/$TARGET_FILE"
      if ! grep -q "Done when" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- TASK 文件缺少 Done when 验收标准"
      fi
      if grep -q "Done when：完成" "$FILE_PATH" || grep -q "Done when：实现" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- TASK 存在模糊的 Done when（'完成'或'实现'不是有效标准）"
      fi
      ;;
    TEST.md)
      FILE_PATH="$PC_DOC_ROOT/TEST.md"
      if ! grep -q "测试用例" "$FILE_PATH" && ! grep -q "| ID" "$FILE_PATH"; then
        ISSUES="$ISSUES\n- TEST 缺少具体测试用例"
      fi
      ;;
  esac

  if [ -n "$ISSUES" ]; then
    echo "[dev-flow] 阶段完成检查发现问题："
    echo -e "$ISSUES"
    echo ""
    echo "请补充以上内容后再推进到下一阶段。"
  fi
fi
