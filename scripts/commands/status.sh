#!/bin/bash
# /status 命令脚本化实现
# 用法：bash status.sh [DOC_ROOT]

DOC_ROOT="${1:-dev-doc}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

DOC_ROOT=$(devflow_resolve_doc_root "$DOC_ROOT")

STATUS_FILE="$DOC_ROOT/STATUS.yaml"
if [ ! -f "$STATUS_FILE" ]; then
  echo "[dev-flow] dev-doc 不存在或未初始化，建议执行 /prd 开始。"
  exit 0
fi

# 读取字段
NAME=$(devflow_yaml_get "$STATUS_FILE" name)
PHASE=$(devflow_yaml_get "$STATUS_FILE" phase)
MODE=$(devflow_yaml_get "$STATUS_FILE" mode)
UPDATED=$(devflow_yaml_get "$STATUS_FILE" updated)

# task/ 统计
TOTAL=0; DONE=0
for f in "$DOC_ROOT/task/task_"*.md "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  FILE_TOTAL=$(grep -c "^- \[" "$f" 2>/dev/null) || true; FILE_TOTAL=${FILE_TOTAL:-0}
  FILE_DONE=$(grep -c "^- \[x\]" "$f" 2>/dev/null) || true; FILE_DONE=${FILE_DONE:-0}
  TOTAL=$((TOTAL + FILE_TOTAL))
  DONE=$((DONE + FILE_DONE))
done

# issue/ 统计
OPEN_ISSUES=0; CLOSED_ISSUES=0
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[ \]" "$f" 2>/dev/null) || true; OPEN_ISSUES=$((OPEN_ISSUES + ${CNT:-0}))
  done
  for f in "$DOC_ROOT/issue/closed_issue_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[" "$f" 2>/dev/null) || true; CLOSED_ISSUES=$((CLOSED_ISSUES + ${CNT:-0}))
  done
fi

# 进度条
if [ "$TOTAL" -gt 0 ]; then
  PCT=$((DONE * 10 / TOTAL))
  BAR=""
  for i in $(seq 1 10); do
    if [ "$i" -le "$PCT" ]; then BAR="${BAR}█"; else BAR="${BAR}░"; fi
  done
  PROGRESS="[$BAR] $DONE/$TOTAL"
else
  PROGRESS="无任务"
fi

# CHANGELOG 最近动态
RECENT=""
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  RECENT=$(grep "^- " "$DOC_ROOT/CHANGELOG.md" | head -3)
fi

# 建议下一步
NEXT=""
HAS_TEST_REPORT=false
TEST_HAS_FAILURE=false
[ -f "$DOC_ROOT/TEST.md" ] && HAS_TEST_REPORT=true
if [ -f "$DOC_ROOT/TEST.md" ] && grep -qE "FAILED SUITES:|FAIL: [1-9][0-9]*|失败: [1-9][0-9]*|失败：[1-9][0-9]*|未通过：[1-9][0-9]*|未通过: [1-9][0-9]*" "$DOC_ROOT/TEST.md"; then
  TEST_HAS_FAILURE=true
fi

case "$PHASE" in
  PRD)  NEXT="/spec" ;;
  SPEC) NEXT="/task" ;;
  TASK) NEXT="确认任务后进入 DEV" ;;
  DEV)
    if [ "$OPEN_ISSUES" -gt 0 ]; then NEXT="/fix"
    elif [ "$DONE" -eq "$TOTAL" ] && [ "$TOTAL" -gt 0 ]; then NEXT="/test"
    else NEXT="继续开发"
    fi ;;
  TEST)
    if [ "$OPEN_ISSUES" -gt 0 ]; then NEXT="/fix"
    elif [ "$HAS_TEST_REPORT" = false ] || [ "$TEST_HAS_FAILURE" = true ]; then NEXT="/test"
    else NEXT="/iterate"
    fi ;;
  DONE) NEXT="/iterate" ;;
esac

# 输出
echo "[dev-flow] 项目状态报告"
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "项目名称：${NAME:-未设置}"
echo "文档根：$DOC_ROOT"
echo "当前阶段：$PHASE"
echo "开发模式：${MODE:-未设置}"
# 版本号从 VERSION 文件读取
if [ -f "VERSION" ]; then
  VER=$(cat VERSION | tr -d '[:space:]')
  TAG_STATUS="未同步"
  if git tag -l "v$VER" 2>/dev/null | grep -q "v$VER"; then
    TAG_STATUS="已同步"
  fi
  echo "当前版本：v${VER}（git tag: ${TAG_STATUS}）"
else
  echo "当前版本：未设置（缺少 VERSION 文件）"
fi
echo "更新时间：${UPDATED:-未知}"
echo ""
echo "任务进度：$PROGRESS"
echo "未关闭 Issue：$OPEN_ISSUES 个"
if [ -n "$RECENT" ]; then
  echo "最近动态："
  echo "$RECENT" | sed 's/^/  /'
fi
echo ""
echo "建议下一步：$NEXT"
