#!/bin/bash
# T14 验证脚本：向后兼容迁移逻辑（/init 自动迁移）
# 验证 scripts/init/migrate.sh 的三项迁移功能

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/t14_test_project"
MIGRATE_SCRIPT="$PROJECT_ROOT/scripts/init/migrate.sh"
TODAY=$(date +%Y-%m-%d)
PASS=0
FAIL=0

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() {
  echo -e "${GREEN}[PASS]${NC} $1"
  PASS=$((PASS + 1))
}

fail() {
  echo -e "${RED}[FAIL]${NC} $1"
  FAIL=$((FAIL + 1))
}

# === 清理并创建测试环境 ===
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
cleanup

echo "=== T14: 向后兼容迁移逻辑验证 ==="
echo "迁移脚本: $MIGRATE_SCRIPT"
echo "测试目录: $TMP_DIR"
echo ""

# === 前提检查 ===
echo "--- 前提检查 ---"

if [ -x "$MIGRATE_SCRIPT" ]; then
  pass "migrate.sh 存在且可执行"
else
  fail "migrate.sh 不存在或不可执行"
  exit 1
fi

# 检查 commands/init.md 引用
if grep -q "migrate" "$PROJECT_ROOT/commands/init.md"; then
  pass "commands/init.md 中有对 migrate 的引用"
else
  fail "commands/init.md 中未引用 migrate"
fi

# === 构建模拟旧项目环境 ===
echo ""
echo "--- 构建模拟旧项目环境 ---"

DOC_ROOT="$TMP_DIR/dev-doc"
mkdir -p "$DOC_ROOT"
mkdir -p "$DOC_ROOT/session"

# 创建旧 TASK.md
cat > "$DOC_ROOT/TASK.md" << 'EOF'
# 任务列表

## T1: 示例任务
- [ ] 步骤 1
- [ ] 步骤 2

## T2: 另一个任务
- [x] 已完成步骤
EOF

# 创建 session 文件
cat > "$DOC_ROOT/session/001-setup.md" << 'EOF'
# Session: Setup
初始化项目结构
EOF

cat > "$DOC_ROOT/session/002-feature.md" << 'EOF'
# Session: Feature
实现核心功能
EOF

# 创建 STATUS.yaml (phase: MVP)
cat > "$DOC_ROOT/STATUS.yaml" << 'EOF'
project: test-project
phase: MVP
iteration: 1
EOF

echo "模拟环境已创建"

# === 执行迁移 ===
echo ""
echo "--- 执行迁移脚本 ---"
bash "$MIGRATE_SCRIPT" "$DOC_ROOT"
echo ""

# === 验证结果 ===
echo "--- 验证迁移结果 ---"

# 验证 1: task/ 目录下有 task_<today>_*.md
if [ -d "$DOC_ROOT/task" ]; then
  pass "task/ 目录已创建"
else
  fail "task/ 目录不存在"
fi

TASK_FILE="$DOC_ROOT/task/task_${TODAY}_1.md"
if [ -f "$TASK_FILE" ]; then
  pass "task/${TODAY}_1.md 文件存在"
else
  fail "task/${TODAY}_1.md 文件不存在（期望: $TASK_FILE）"
fi

# 验证任务文件内容与原始 TASK.md 一致
if [ -f "$TASK_FILE" ] && grep -q "示例任务" "$TASK_FILE"; then
  pass "迁移后的任务文件内容正确"
else
  fail "迁移后的任务文件内容不正确"
fi

# 验证 2: TASK.md.bak 存在
if [ -f "$DOC_ROOT/TASK.md.bak" ]; then
  pass "TASK.md.bak 备份文件存在"
else
  fail "TASK.md.bak 备份文件不存在"
fi

# 验证原始 TASK.md 已被移走
if [ ! -f "$DOC_ROOT/TASK.md" ]; then
  pass "原始 TASK.md 已被移除"
else
  fail "原始 TASK.md 仍然存在（应已被 mv 为 .bak）"
fi

# 验证 3: CHANGELOG.md 被创建
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  pass "CHANGELOG.md 已创建"
else
  fail "CHANGELOG.md 未创建"
fi

# 验证 CHANGELOG.md 包含 session 摘要
if grep -q "setup" "$DOC_ROOT/CHANGELOG.md" && grep -q "feature" "$DOC_ROOT/CHANGELOG.md"; then
  pass "CHANGELOG.md 包含 session 摘要内容"
else
  fail "CHANGELOG.md 缺少 session 摘要"
fi

# 验证 4: STATUS.yaml 的 phase 为 DEV
PHASE=$(grep "^phase:" "$DOC_ROOT/STATUS.yaml" | sed 's/^phase: *//')
if [ "$PHASE" = "DEV" ]; then
  pass "STATUS.yaml phase 已从 MVP 更新为 DEV"
else
  fail "STATUS.yaml phase 未正确更新（当前值: '$PHASE'，期望: 'DEV'）"
fi

# === 汇总 ===
echo ""
echo "=== 测试结果 ==="
echo -e "通过: ${GREEN}${PASS}${NC}"
echo -e "失败: ${RED}${FAIL}${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
  echo -e "${GREEN}结论：通过${NC}"
  exit 0
else
  echo -e "${RED}结论：未通过${NC}"
  exit 1
fi
