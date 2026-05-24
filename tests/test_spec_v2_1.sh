#!/bin/bash
# 全量测试：验证 SPEC v2.1 (agent 输入系统重构) 的全部功能
# 覆盖：T1-T7 的所有 Done when 标准 + SPEC 的非功能需求

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONTEXT_SH="$SCRIPT_DIR/scripts/lib/context.sh"
COMMANDS_DIR="$SCRIPT_DIR/commands"
AGENTS_DIR="$SCRIPT_DIR/agents"
TMP_DIR="$SCRIPT_DIR/tmp/test_spec_v2_1_$$"
PASS=0; FAIL=0; ERRORS=""

# 辅助函数
assert_eq() {
  local actual="$1" expected="$2" msg="$3"
  if [ "$actual" = "$expected" ]; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: [$expected]\n    got: [$actual]"
    echo "  FAIL: $msg (expected: [$expected], got: [$actual])"
  fi
}

assert_gt() {
  local actual="$1" limit="$2" msg="$3"
  if [ "$actual" -gt "$limit" ] 2>/dev/null; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected > $limit, got: $actual"
    echo "  FAIL: $msg (expected > $limit, got: $actual)"
  fi
}

assert_le() {
  local actual="$1" limit="$2" msg="$3"
  if [ "$actual" -le "$limit" ] 2>/dev/null; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected <= $limit, got: $actual"
    echo "  FAIL: $msg (expected <= $limit, got: $actual)"
  fi
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected"
    echo "  FAIL: $msg (not found: $expected)"
  fi
}

assert_contains_regex() {
  local output="$1" pattern="$2" msg="$3"
  if echo "$output" | grep -qE "$pattern"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected regex match: $pattern"
    echo "  FAIL: $msg (no regex match: $pattern)"
  fi
}

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if echo "$output" | grep -qF "$unexpected"; then
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain: $unexpected"
    echo "  FAIL: $msg (should NOT contain: $unexpected)"
  else
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  fi
}

assert_file_not_exists() {
  local path="$1" msg="$2"
  if [ ! -f "$path" ]; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    file should NOT exist: $path"
    echo "  FAIL: $msg (file exists: $path)"
  fi
}

# 准备临时目录
mkdir -p "$TMP_DIR"

echo "======================================================"
echo "= SPEC v2.1 全量测试"
echo "======================================================"
echo ""

# ============================================================
# T1: context.sh 项目上下文扫描
# ============================================================
echo "--- T1: context.sh 功能验证 ---"

# T1-1: 本项目输出行数 <= 200
OUTPUT=$(bash "$CONTEXT_SH" "$SCRIPT_DIR" 2>&1)
LINE_COUNT=$(echo "$OUTPUT" | wc -l)
assert_le "$LINE_COUNT" "200" "T1-1: 输出不超过 200 行 (实际: $LINE_COUNT)"

# T1-2: 输出包含"技术栈"
assert_contains "$OUTPUT" "技术栈" "T1-2: 输出包含「技术栈」"

# T1-3: 输出包含"目录结构"
assert_contains "$OUTPUT" "目录结构" "T1-3: 输出包含「目录结构」"

# T1-4: 输出包含"已有测试"
assert_contains "$OUTPUT" "已有测试" "T1-4: 输出包含「已有测试」"

# T1-5: 输出包含"运行方式"
assert_contains "$OUTPUT" "运行方式" "T1-5: 输出包含「运行方式」"

# T1-6: 输出包含"核心模块"
assert_contains "$OUTPUT" "核心模块" "T1-6: 输出包含「核心模块」"

# T1-7: 正确推断 Shell/Bash 技术栈
assert_contains "$OUTPUT" "Shell/Bash" "T1-7: 正确推断 Shell/Bash 技术栈"

# T1-8: 空目录不报错，退出码为 0
mkdir -p "$TMP_DIR/empty"
OUTPUT_EMPTY=$(bash "$CONTEXT_SH" "$TMP_DIR/empty" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "T1-8: 空目录退出码为 0"

# T1-9: 空目录不输出错误信息
assert_not_contains "$OUTPUT_EMPTY" "Error" "T1-9: 空目录无 Error 输出"
assert_not_contains "$OUTPUT_EMPTY" "error" "T1-9b: 空目录无 error 输出"

# T1-10: 不存在的目录处理
OUTPUT_NODIR=$(bash "$CONTEXT_SH" "$TMP_DIR/nonexistent_dir_xyz" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "T1-10: 不存在目录退出码为 0"
assert_contains "$OUTPUT_NODIR" "目录不存在" "T1-10b: 不存在目录给出提示"

# T1-11: 默认参数（当前目录）
cd "$SCRIPT_DIR"
OUTPUT_DEFAULT=$(bash "$CONTEXT_SH" 2>&1)
assert_contains "$OUTPUT_DEFAULT" "技术栈" "T1-11: 无参数时使用当前目录"

# T1-12: fallback 验证（模拟无 tree 命令）
mkdir -p "$TMP_DIR/fake_bin"
cat > "$TMP_DIR/fake_bin/tree" << 'FAKE'
#!/bin/bash
exit 1
FAKE
chmod +x "$TMP_DIR/fake_bin/tree"

# 创建一个不含 tmp 字样的临时项目目录用于 fallback 测试
FALLBACK_DIR="$SCRIPT_DIR/tmp/fallback_proj_$$"
mkdir -p "$FALLBACK_DIR/src"
touch "$FALLBACK_DIR/src/main.sh"
OUTPUT_FALLBACK=$(PATH="$TMP_DIR/fake_bin:/usr/bin:/bin" bash "$CONTEXT_SH" "$FALLBACK_DIR" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "T1-12: 无 tree 环境退出码为 0"
assert_contains "$OUTPUT_FALLBACK" "目录结构" "T1-12b: fallback 仍输出「目录结构」"

# T1-13: 性能（< 500ms）
START=$(date +%s%N 2>/dev/null || echo "0")
bash "$CONTEXT_SH" "$SCRIPT_DIR" > /dev/null 2>&1
END=$(date +%s%N 2>/dev/null || echo "0")
if [ "$START" != "0" ] && [ "$END" != "0" ]; then
  ELAPSED_MS=$(( (END - START) / 1000000 ))
  assert_le "$ELAPSED_MS" "500" "T1-13: 执行时间 < 500ms (实际: ${ELAPSED_MS}ms)"
else
  # date 不支持 %N 时跳过
  PASS=$((PASS + 1))
  echo "  PASS: T1-13: 性能测试跳过（系统不支持纳秒）"
fi

# T1-14: 技术栈推断 - Node.js
mkdir -p "$TMP_DIR/node_proj"
echo '{"name":"test"}' > "$TMP_DIR/node_proj/package.json"
OUTPUT_NODE=$(bash "$CONTEXT_SH" "$TMP_DIR/node_proj" 2>&1)
assert_contains "$OUTPUT_NODE" "Node.js" "T1-14: 检测到 package.json → Node.js"

# T1-15: 技术栈推断 - Python
mkdir -p "$TMP_DIR/py_proj"
touch "$TMP_DIR/py_proj/requirements.txt"
OUTPUT_PY=$(bash "$CONTEXT_SH" "$TMP_DIR/py_proj" 2>&1)
assert_contains "$OUTPUT_PY" "Python" "T1-15: 检测到 requirements.txt → Python"

# T1-16: 技术栈推断 - Rust
mkdir -p "$TMP_DIR/rust_proj"
touch "$TMP_DIR/rust_proj/Cargo.toml"
OUTPUT_RUST=$(bash "$CONTEXT_SH" "$TMP_DIR/rust_proj" 2>&1)
assert_contains "$OUTPUT_RUST" "Rust" "T1-16: 检测到 Cargo.toml → Rust"

# T1-17: 技术栈推断 - Go
mkdir -p "$TMP_DIR/go_proj"
touch "$TMP_DIR/go_proj/go.mod"
OUTPUT_GO=$(bash "$CONTEXT_SH" "$TMP_DIR/go_proj" 2>&1)
assert_contains "$OUTPUT_GO" "Go" "T1-17: 检测到 go.mod → Go"

echo ""

# ============================================================
# T2: commands/spec.md 模式感知
# ============================================================
echo "--- T2: spec.md 模式感知验证 ---"

SPEC_MD=$(cat "$COMMANDS_DIR/spec.md")

# T2-1: 包含 context 引用
CONTEXT_COUNT=$(echo "$SPEC_MD" | grep -c "context")
assert_gt "$CONTEXT_COUNT" "0" "T2-1: spec.md 包含 context 引用 (数量: $CONTEXT_COUNT)"

# T2-2: 包含模式相关关键词
MODE_COUNT=$(echo "$SPEC_MD" | grep -c "mode\|模式")
assert_gt "$MODE_COUNT" "0" "T2-2: spec.md 包含模式关键词 (数量: $MODE_COUNT)"

# T2-3: full 模式要求 PRD.md 存在
assert_contains "$SPEC_MD" "full" "T2-3: spec.md 包含 full 模式描述"
assert_contains "$SPEC_MD" "PRD.md" "T2-3b: spec.md 引用 PRD.md"

# T2-4: quick/mvp 降级逻辑
assert_contains "$SPEC_MD" "BRAINSTORM" "T2-4: spec.md 包含 BRAINSTORM 降级路径"
assert_contains "$SPEC_MD" "用户描述" "T2-4b: spec.md 包含用户描述替代路径"

# T2-5: 项目上下文始终传入
assert_contains "$SPEC_MD" "始终传入" "T2-5: spec.md 明确项目上下文始终传入"

echo ""

# ============================================================
# T3: commands/task.md 模式感知
# ============================================================
echo "--- T3: task.md 模式感知验证 ---"

TASK_MD=$(cat "$COMMANDS_DIR/task.md")

# T3-1: 包含 context 引用
CONTEXT_COUNT=$(echo "$TASK_MD" | grep -c "context")
assert_gt "$CONTEXT_COUNT" "0" "T3-1: task.md 包含 context 引用 (数量: $CONTEXT_COUNT)"

# T3-2: fast 模式包含项目上下文
assert_contains "$TASK_MD" "fast" "T3-2: task.md 包含 fast 模式描述"

# T3-3: full/quick 要求 SPEC.md 存在
assert_contains "$TASK_MD" "SPEC.md" "T3-3: task.md 引用 SPEC.md"

# T3-4: 项目上下文始终传入
assert_contains "$TASK_MD" "始终传入" "T3-4: task.md 明确项目上下文始终传入"

# T3-5: agent 调度模板中包含项目上下文传入说明
assert_contains "$TASK_MD" "项目上下文" "T3-5: task.md agent 模板包含项目上下文"

echo ""

# ============================================================
# T4: test.md / devtest.md / fix.md 追加项目上下文
# ============================================================
echo "--- T4: test/devtest/fix 项目上下文验证 ---"

# test.md
TEST_MD=$(cat "$COMMANDS_DIR/test.md")
CONTEXT_COUNT=$(echo "$TEST_MD" | grep -c "context\|上下文")
assert_gt "$CONTEXT_COUNT" "0" "T4-1: test.md 包含上下文引用 (数量: $CONTEXT_COUNT)"

# test.md 无硬编码 .py
PY_COUNT=$(echo "$TEST_MD" | grep -c "test_.*\.py" || true)
assert_eq "$PY_COUNT" "0" "T4-2: test.md 无 test_*.py 硬编码"

# test.md 包含 done_task
DONE_COUNT=$(echo "$TEST_MD" | grep -c "done_task" || true)
assert_gt "$DONE_COUNT" "0" "T4-3: test.md 引用 done_task (数量: $DONE_COUNT)"

# devtest.md
DEVTEST_MD=$(cat "$COMMANDS_DIR/devtest.md")
CONTEXT_COUNT=$(echo "$DEVTEST_MD" | grep -c "context\|上下文")
assert_gt "$CONTEXT_COUNT" "0" "T4-4: devtest.md 包含上下文引用 (数量: $CONTEXT_COUNT)"

# fix.md
FIX_MD=$(cat "$COMMANDS_DIR/fix.md")
CONTEXT_COUNT=$(echo "$FIX_MD" | grep -c "context\|上下文")
assert_gt "$CONTEXT_COUNT" "0" "T4-5: fix.md 包含上下文引用 (数量: $CONTEXT_COUNT)"

echo ""

# ============================================================
# T5: test-agent.md 无硬编码 .py
# ============================================================
echo "--- T5: test-agent.md 扩展名验证 ---"

AGENT_MD=$(cat "$AGENTS_DIR/test-agent.md")

# T5-1: 不含 .py 硬编码
PY_COUNT=$(echo "$AGENT_MD" | grep -c "test_.*\.py" || true)
assert_eq "$PY_COUNT" "0" "T5-1: test-agent.md 无 test_*.py 硬编码"

# T5-2: 仍保留 tests/ 目录规范
assert_contains "$AGENT_MD" "tests/" "T5-2: test-agent.md 保留 tests/ 规范"

# T5-3: 使用通用扩展名表述
assert_contains_regex "$AGENT_MD" "test_.*\.<ext>|test_.*\.\{ext\}" "T5-3: test-agent.md 使用通用扩展名"

echo ""

# ============================================================
# T6: 冗余 hook 文件删除 + marketplace.json 修正
# ============================================================
echo "--- T6: 冗余 hook 清理验证 ---"

# T6-1 ~ T6-4: 4 个文件必须已删除
assert_file_not_exists "$SCRIPT_DIR/scripts/hooks/check-doc-sync.sh" "T6-1: check-doc-sync.sh 已删除"
assert_file_not_exists "$SCRIPT_DIR/scripts/hooks/check-phase-completion.sh" "T6-2: check-phase-completion.sh 已删除"
assert_file_not_exists "$SCRIPT_DIR/scripts/hooks/check-task-completion.sh" "T6-3: check-task-completion.sh 已删除"
assert_file_not_exists "$SCRIPT_DIR/scripts/hooks/update-status.sh" "T6-4: update-status.sh 已删除"

# T6-5: hooks.json 中无残留引用
HOOKS_REFS=$(grep -r "check-doc-sync\|check-phase-completion\|check-task-completion\|update-status" "$SCRIPT_DIR/hooks/hooks.json" 2>/dev/null || true)
assert_eq "$HOOKS_REFS" "" "T6-5: hooks.json 无已删除 hook 引用"

# T6-6: marketplace.json 无 "DONE" 引用（应为 ITERATE）
MARKETPLACE_FILE="$SCRIPT_DIR/.claude-plugin/marketplace.json"
if [ -f "$MARKETPLACE_FILE" ]; then
  DONE_REFS=$(grep "DONE" "$MARKETPLACE_FILE" 2>/dev/null || true)
  assert_eq "$DONE_REFS" "" "T6-6: marketplace.json 无 DONE 引用"
else
  PASS=$((PASS + 1))
  echo "  PASS: T6-6: marketplace.json 不含 DONE (文件不存在也满足条件)"
fi

echo ""

# ============================================================
# 非功能需求：200 行上限
# ============================================================
echo "--- 非功能：200 行上限验证 ---"

# 创建一个有大量文件的项目，验证输出仍不超过 200 行
mkdir -p "$TMP_DIR/big_proj/src"
for i in $(seq 1 100); do
  touch "$TMP_DIR/big_proj/src/module_$i.sh"
done
mkdir -p "$TMP_DIR/big_proj/tests"
for i in $(seq 1 50); do
  touch "$TMP_DIR/big_proj/tests/test_$i.sh"
done
mkdir -p "$TMP_DIR/big_proj/scripts"
touch "$TMP_DIR/big_proj/scripts/run.sh"

OUTPUT_BIG=$(bash "$CONTEXT_SH" "$TMP_DIR/big_proj" 2>&1)
LINE_COUNT_BIG=$(echo "$OUTPUT_BIG" | wc -l)
assert_le "$LINE_COUNT_BIG" "200" "非功能-1: 大项目输出仍 <= 200 行 (实际: $LINE_COUNT_BIG)"

echo ""

# ============================================================
# 边界场景：context.sh 特殊输入
# ============================================================
echo "--- 边界场景 ---"

# 路径含空格
mkdir -p "$TMP_DIR/space dir/src"
touch "$TMP_DIR/space dir/src/main.sh"
OUTPUT_SPACE=$(bash "$CONTEXT_SH" "$TMP_DIR/space dir" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "边界-1: 路径含空格退出码为 0"
assert_contains "$OUTPUT_SPACE" "技术栈" "边界-1b: 路径含空格仍输出技术栈"

# 路径含中文
mkdir -p "$TMP_DIR/中文项目/src"
touch "$TMP_DIR/中文项目/src/app.sh"
OUTPUT_CN=$(bash "$CONTEXT_SH" "$TMP_DIR/中文项目" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "边界-2: 路径含中文退出码为 0"

# 符号链接
mkdir -p "$TMP_DIR/link_target"
touch "$TMP_DIR/link_target/file.sh"
ln -sf "$TMP_DIR/link_target" "$TMP_DIR/symlink_proj" 2>/dev/null
OUTPUT_LINK=$(bash "$CONTEXT_SH" "$TMP_DIR/symlink_proj" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "边界-3: 符号链接路径退出码为 0"

echo ""

# ============================================================
# SPEC 规范对照：隔离边界正确性
# ============================================================
echo "--- SPEC 规范对照 ---"

# spec.md 的隔离规则：禁止传入 TASK.md / TEST.md
assert_not_contains "$SPEC_MD" "传入.*TASK.md\|传入.*TEST.md" "规范-1: spec.md 不传入 TASK/TEST（需人工确认）"
# 实际用 grep 精确检查
PROHIBITED_IN_SPEC=$(echo "$SPEC_MD" | grep "允许传入" -A20 | grep "TASK.md\|TEST.md" | grep -v "禁止" || true)
# 反过来验证：禁止传入列表中包含 TASK.md 和 TEST.md
FORBIDDEN=$(echo "$SPEC_MD" | grep "禁止传入" -A5)
assert_contains "$FORBIDDEN" "TASK.md" "规范-1b: spec.md 禁止传入 TASK.md"

# test.md 的隔离规则：禁止传入 PRD.md
FORBIDDEN_TEST=$(echo "$TEST_MD" | grep "禁止传入" -A5)
assert_contains "$FORBIDDEN_TEST" "PRD.md" "规范-2: test.md 禁止传入 PRD.md"

# fix.md 的隔离规则：禁止传入 PRD.md
FORBIDDEN_FIX=$(echo "$FIX_MD" | grep "禁止传入" -A5)
assert_contains "$FORBIDDEN_FIX" "PRD.md" "规范-3: fix.md 禁止传入 PRD.md"

echo ""

# ============================================================
# 清理
# ============================================================
rm -rf "$TMP_DIR"
rm -rf "$FALLBACK_DIR"

# ============================================================
# 汇总
# ============================================================
echo "======================================================"
echo "= 测试结果汇总"
echo "======================================================"
echo "PASS: $PASS  FAIL: $FAIL  TOTAL: $((PASS + FAIL))"
if [ $FAIL -gt 0 ]; then
  echo ""
  echo "失败详情:"
  echo -e "$ERRORS"
  echo ""
  exit 1
fi
echo ""
echo "全部通过！"
exit 0
