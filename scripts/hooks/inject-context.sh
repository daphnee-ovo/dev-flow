#!/bin/bash
# Hook: UserPromptSubmit
# 注入当前项目阶段上下文 + 任务/issue 统计 + 规范提醒

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

PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
if [ -z "$PHASE" ]; then
  exit 0
fi

MODE=$(grep "^mode:" "$STATUS_FILE" | sed 's/^mode: *//')

# === 从 task/ 目录统计任务 ===
TOTAL=0; DONE=0; P0_TOTAL=0; P1_TOTAL=0; P2_TOTAL=0
P0_UNDONE=0; P1_UNDONE=0; P2_UNDONE=0

for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; TOTAL=$((TOTAL + ${CNT:-0}))
  CNT=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; DONE=$((DONE + ${CNT:-0}))
  CNT=$(grep -c "level: P0" "$f" 2>/dev/null) || true; P0_TOTAL=$((P0_TOTAL + ${CNT:-0}))
  CNT=$(grep -c "level: P1" "$f" 2>/dev/null) || true; P1_TOTAL=$((P1_TOTAL + ${CNT:-0}))
  CNT=$(grep -c "level: P2" "$f" 2>/dev/null) || true; P2_TOTAL=$((P2_TOTAL + ${CNT:-0}))
done
# done_ 文件也计入总数
for f in "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true
  TOTAL=$((TOTAL + ${CNT:-0})); DONE=$((DONE + ${CNT:-0}))
  CNT=$(grep -c "level: P0" "$f" 2>/dev/null) || true; P0_TOTAL=$((P0_TOTAL + ${CNT:-0}))
  CNT=$(grep -c "level: P1" "$f" 2>/dev/null) || true; P1_TOTAL=$((P1_TOTAL + ${CNT:-0}))
  CNT=$(grep -c "level: P2" "$f" 2>/dev/null) || true; P2_TOTAL=$((P2_TOTAL + ${CNT:-0}))
done

# === 统计未关闭 issue ===
OPEN_ISSUES=0
OPEN_ISSUE_FILES=""
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    UNCHECKED=$(grep -c "^- \[ \]" "$f" 2>/dev/null) || true
    UNCHECKED=${UNCHECKED:-0}
    if [ "$UNCHECKED" -gt 0 ]; then
      OPEN_ISSUES=$((OPEN_ISSUES + UNCHECKED))
      OPEN_ISSUE_FILES="$OPEN_ISSUE_FILES $f"
    fi
  done
fi

# === 基础状态输出 ===
echo "[dev-flow ${MODE:-?}] STAGE: $PHASE | TASK: $DONE/$TOTAL | ISSUE: $OPEN_ISSUES"

# === 阶段 HINTS ===
case "$PHASE" in
  PRD)  echo "[PRD HINTS] 完成后 /spec" ;;
  SPEC) echo "[SPEC HINTS] 定义接口+数据模型+错误处理 → /task" ;;
  TASK) echo "[TASK HINTS] 每任务必须有 Done when → 进入 DEV" ;;
  DEV)
    # 阻断：DEV 阶段必须有活跃 task 或 open issue 才能推进
    ACTIVE_TASKS=0
    for f in "$DOC_ROOT/task/task_"*.md; do
      [ -f "$f" ] && ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
    done
    if [ "$ACTIVE_TASKS" -eq 0 ] && [ "$OPEN_ISSUES" -eq 0 ]; then
      echo "[BLOCKED] DEV 阶段无活跃 task 且无 open issue，无法推进。"
      echo "[BLOCKED] 请先执行 /task 创建任务，或 /issue 创建 issue。"
      exit 0
    fi
    echo "[DEV HINTS] task/ 任务 → 勾选[x] → /devtest → tests/ | issue → /fix"
    [ "$OPEN_ISSUES" -gt 0 ] && echo "[!] $OPEN_ISSUES issue open → /fix"
    ;;
  TEST)
    echo "[TEST HINTS] 运行 tests/ 全量 → issue 或 /done"
    [ "$OPEN_ISSUES" -gt 0 ] && echo "[!] $OPEN_ISSUES issue open → /fix first"
    ;;
  DONE) echo "[DONE HINTS] /iterate 启动新迭代" ;;
esac

# === issue/task 互斥展示 ===
if [ "$OPEN_ISSUES" -gt 0 ]; then
  # 有 issue 时展示 issue 标题（按优先级分层）
  CURRENT_SEV=""
  for SEV in P0 P1 P2; do
    HAS_OPEN=0
    for f in $OPEN_ISSUE_FILES; do
      [ -f "$f" ] || continue
      # 检查是否有该优先级的未关闭 issue
      if awk '/^- \[ \]/{title=$0; for(i=1;i<=4;i++){if(!getline)break; if(/severity: '"$SEV"'/){found=1;break}; if(/^- \[/)break}} END{exit !found}' "$f" 2>/dev/null; then
        HAS_OPEN=1
        break
      fi
    done
    if [ "$HAS_OPEN" -eq 1 ]; then
      CURRENT_SEV="$SEV"
      break
    fi
  done

  if [ -n "$CURRENT_SEV" ]; then
    echo "[$CURRENT_SEV ISSUE LIST]"
    for f in $OPEN_ISSUE_FILES; do
      [ -f "$f" ] || continue
      awk '/^- \[ \]/{title=$0; for(i=1;i<=4;i++){if(!getline)break; if(/severity: '"$CURRENT_SEV"'/){print "  " title;break}; if(/^- \[/)break}}' "$f" 2>/dev/null | sed 's/^  - \[ \] /  - /'
    done
  fi
elif [ "$TOTAL" -gt 0 ]; then
  # 无 issue 时展示当前优先级的 task 标题
  CURRENT_LEVEL=""
  for LEVEL in P0 P1 P2; do
    HAS_UNDONE=0
    for f in "$DOC_ROOT/task/task_"*.md; do
      [ -f "$f" ] || continue
      if awk '/^- \[ \]/{title=$0; for(i=1;i<=4;i++){if(!getline)break; if(/level: '"$LEVEL"'/){found=1;break}; if(/^- \[/)break}} END{exit !found}' "$f" 2>/dev/null; then
        HAS_UNDONE=1
        break
      fi
    done
    if [ "$HAS_UNDONE" -eq 1 ]; then
      CURRENT_LEVEL="$LEVEL"
      break
    fi
  done

  if [ -n "$CURRENT_LEVEL" ]; then
    echo "[$CURRENT_LEVEL TASK LIST]"
    for f in "$DOC_ROOT/task/task_"*.md; do
      [ -f "$f" ] || continue
      awk '/^- \[ \]/{title=$0; for(i=1;i<=4;i++){if(!getline)break; if(/level: '"$CURRENT_LEVEL"'/){print "  " title;break}; if(/^- \[/)break}}' "$f" 2>/dev/null | sed 's/^  - \[ \] /  - /'
    done
  fi
fi

# === 最近一条 CHANGELOG ===
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  LAST_ENTRY=$(grep -a "^- " "$DOC_ROOT/CHANGELOG.md" | head -1)
  [ -n "$LAST_ENTRY" ] && echo "[LAST] $LAST_ENTRY"
fi
