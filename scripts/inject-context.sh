#!/bin/bash
# Hook: UserPromptSubmit
# 每次用户发消息时，注入当前项目阶段上下文 + 规范提醒
# 让当前 agent 永远知道项目在哪个阶段、该做什么、遵守什么规范

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# 确定文档根目录
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.md" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

STATUS_FILE="$DOC_ROOT/STATUS.md"
if [ ! -f "$STATUS_FILE" ]; then
  exit 0
fi

# 读取当前阶段
PHASE=$(grep "当前阶段" "$STATUS_FILE" | sed 's/.*：//' | tr -d ' ')
if [ -z "$PHASE" ]; then
  exit 0
fi

# 读取开发模式
MODE=$(grep "开发模式" "$STATUS_FILE" | sed 's/.*：//' | tr -d ' ')

# 读取任务进度
TOTAL=0
DONE=0
if [ -f "$DOC_ROOT/TASK.md" ]; then
  TOTAL=$(grep -c "^- \[" "$DOC_ROOT/TASK.md" 2>/dev/null || echo 0)
  DONE=$(grep -c "^- \[x\]" "$DOC_ROOT/TASK.md" 2>/dev/null || echo 0)
fi

# 统计未关闭 issue
OPEN_ISSUES=0
if [ -d "$DOC_ROOT/issue" ]; then
  OPEN_ISSUES=$(find "$DOC_ROOT/issue" -name "issue_*.md" ! -name "closed_issue_*.md" 2>/dev/null | wc -l)
fi

# === 基础状态 ===
echo "[dev-flow] 阶段：$PHASE | 模式：${MODE:-未设置} | 任务：$DONE/$TOTAL | Issue：$OPEN_ISSUES"

# === B: 通用规范（始终注入） ===
echo "[规范] issue 命名：issue_<source>_<YYYY-MM-DD>_<seq>.md，关闭加 closed_ 前缀"
echo "[规范] 测试代码写入 tests/，不允许终端临时验证"
echo "[规范] 临时文件放项目 tmp/，禁止使用系统 /tmp/，不进 dev-doc/ 或 src/"
echo "[规范] session 命名：<3位序号>-<topic>.md"

# === C: 阶段特定规则 ===
case "$PHASE" in
  PRD)
    echo "[PRD] 需求探索阶段。完成后用 /spec 推进。"
    if [ -f "$DOC_ROOT/BRAINSTORM.md" ]; then
      echo "[PRD] 已有 BRAINSTORM.md，/prd 将基于其内容格式化。"
    fi
    ;;
  SPEC)
    echo "[SPEC] 技术规范阶段。必须定义接口、数据模型、错误处理。完成后用 /task。"
    ;;
  TASK)
    echo "[TASK] 任务拆解阶段。每个任务必须有 Done when 标准。完成后进入开发。"
    ;;
  DEV)
    echo "[DEV] 开发阶段规则："
    echo "  1. 只做 TASK.md 列出的任务"
    echo "  2. 完成一个 → 勾选 [x] → 立即 /devtest"
    echo "  3. /devtest 必须将测试写入 tests/"
    echo "  4. 未通过 → 取消勾选，issue 写入 $DOC_ROOT/issue/"
    if [ "$OPEN_ISSUES" -gt 0 ]; then
      echo "  ⚠ $OPEN_ISSUES 个未关闭 issue，用 /fix 修复"
    fi
    ;;
  TEST)
    echo "[TEST] 全量测试阶段。运行 tests/ 全部用例。"
    echo "  - 失败项写入 issue，通过后用 /done 交付"
    if [ "$OPEN_ISSUES" -gt 0 ]; then
      echo "  ⚠ $OPEN_ISSUES 个未关闭 issue 需先修复"
    fi
    ;;
  DONE)
    echo "[DONE] 已交付。如需继续开发用 /iterate 启动新迭代。"
    ;;
  MVP)
    echo "[MVP] MVP 模式。快速验证核心假设，跳过非关键步骤。"
    ;;
esac
