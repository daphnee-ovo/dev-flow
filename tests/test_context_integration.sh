#!/bin/bash
# 集成测试：context.sh + command 文件联动验证

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONTEXT_SH="$SCRIPT_DIR/scripts/lib/context.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_context_integration_$$"
PASS=0; FAIL=0; ERRORS=""

assert_eq() {
  local actual="$1" expected="$2" msg="$3"
  if [ "$actual" = "$expected" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: $expected\n    got: $actual"
  fi
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected\n    got: $(echo "$output" | head -10)"
  fi
}

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if echo "$output" | grep -qF "$unexpected"; then
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain: $unexpected"
  else
    PASS=$((PASS + 1))
  fi
}

assert_le() {
  local actual="$1" limit="$2" msg="$3"
  if [ "$actual" -le "$limit" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected <= $limit, got: $actual"
  fi
}

# ===== TEST 1: context.sh 在本项目中正确输出且不超过 200 行 =====
echo "TEST 1: context.sh 本项目输出"
OUTPUT=$(bash "$CONTEXT_SH" "$SCRIPT_DIR" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "本项目执行退出码应为 0"
LINE_COUNT=$(echo "$OUTPUT" | wc -l)
assert_le "$LINE_COUNT" "200" "输出不应超过 200 行（实际 $LINE_COUNT 行）"
assert_contains "$OUTPUT" "技术栈" "输出应包含「技术栈」"
assert_contains "$OUTPUT" "目录结构" "输出应包含「目录结构」"

# ===== TEST 2: context.sh 在空目录中输出合理内容且退出码为 0 =====
echo "TEST 2: context.sh 空目录输出"
mkdir -p "$TMP_DIR/empty"
OUTPUT=$(bash "$CONTEXT_SH" "$TMP_DIR/empty" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "空目录执行退出码应为 0"
assert_not_contains "$OUTPUT" "Error" "空目录不应输出 Error"

# ===== TEST 3: 模拟无 tree 命令环境验证 fallback =====
echo "TEST 3: 无 tree 命令 fallback"
mkdir -p "$TMP_DIR/no_tree/src"
touch "$TMP_DIR/no_tree/src/main.sh"
touch "$TMP_DIR/no_tree/README.md"
# 创建一个临时 bin 目录，放入假的 tree 命令（返回空）让 context.sh 走 fallback
mkdir -p "$TMP_DIR/fake_bin"
cat > "$TMP_DIR/fake_bin/tree" << 'FAKE'
#!/bin/bash
exit 1
FAKE
chmod +x "$TMP_DIR/fake_bin/tree"
OUTPUT=$(PATH="$TMP_DIR/fake_bin:/usr/bin:/bin" bash "$CONTEXT_SH" "$TMP_DIR/no_tree" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "无 tree 环境退出码应为 0"
assert_contains "$OUTPUT" "目录结构" "fallback 仍应输出「目录结构」"

# ===== TEST 4: 各 command 文件中确实引用了项目上下文 =====
echo "TEST 4: command 文件引用项目上下文"
COMMANDS_DIR="$SCRIPT_DIR/commands"

# spec.md
MATCH=$(grep -c "context\|上下文" "$COMMANDS_DIR/spec.md")
assert_le "1" "$MATCH" "spec.md 应引用项目上下文（匹配数: $MATCH）"

# task.md
MATCH=$(grep -c "context\|上下文" "$COMMANDS_DIR/task.md")
assert_le "1" "$MATCH" "task.md 应引用项目上下文（匹配数: $MATCH）"

# test.md
MATCH=$(grep -c "context\|上下文" "$COMMANDS_DIR/test.md")
assert_le "1" "$MATCH" "test.md 应引用项目上下文（匹配数: $MATCH）"

# devtest.md
MATCH=$(grep -c "context\|上下文" "$COMMANDS_DIR/devtest.md")
assert_le "1" "$MATCH" "devtest.md 应引用项目上下文（匹配数: $MATCH）"

# fix.md
MATCH=$(grep -c "context\|上下文" "$COMMANDS_DIR/fix.md")
assert_le "1" "$MATCH" "fix.md 应引用项目上下文（匹配数: $MATCH）"

# ===== TEST 5: test.md 不含硬编码 .py =====
echo "TEST 5: test.md 无硬编码 .py"
PY_MATCH=$(grep -c "test_.*\.py" "$COMMANDS_DIR/test.md" || true)
assert_eq "$PY_MATCH" "0" "test.md 不应有 test_*.py 硬编码"

# ===== TEST 6: test.md 包含 done_task 引用 =====
echo "TEST 6: test.md 包含 done_task"
DONE_MATCH=$(grep -c "done_task" "$COMMANDS_DIR/test.md" || true)
assert_le "1" "$DONE_MATCH" "test.md 应引用 done_task（匹配数: $DONE_MATCH）"

# ===== 清理 =====
rm -rf "$TMP_DIR"

# ===== 汇总 =====
echo ""
echo "=== context_integration 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
