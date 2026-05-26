#!/bin/bash
# 验证 /devtest 最小三状态闭环

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/temp/test_devtest_minimal_$$"
SCRIPT="$PROJECT_ROOT/scripts/commands/devtest.sh"
PASS=0
FAIL=0
ERRORS=""

setup() {
  mkdir -p "$TMP_DIR/dev-doc/task"
  cd "$TMP_DIR"
  git init -q .
  cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: quick
updated: 2026-05-26 00:00
started: 2026-05-26 00:00
EOF
  cat > dev-doc/task/task_2026-05-26_1.md << 'EOF'
---
title: TASK - devtest
nums: 1
---

- [x] TASK-T001: 已完成任务
  - priority: P0
  - refs: SPEC-AC-001
  - files:
      test: ["tests/test_sample.sh"]
  - depends_on: []
  - complexity: S
  - done_when:
      - devtest PASS
EOF
}

cleanup() {
  cd "$PROJECT_ROOT"
  if command -v trash >/dev/null 2>&1; then
    trash "$TMP_DIR" >/dev/null 2>&1 || true
  elif [ -d "$TMP_DIR" ]; then
    mv "$TMP_DIR" "$PROJECT_ROOT/temp/test_devtest_minimal_done_$$" 2>/dev/null || true
  fi
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: $expected\n    got: $output"
  fi
}

assert_file_contains() {
  local file="$1" expected="$2" msg="$3"
  if [ -f "$file" ] && grep -qF -- "$expected" "$file"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected in $file: $expected"
  fi
}

echo "TEST 1: PASS 保持勾选并提示 /test"
setup
OUTPUT=$(bash "$SCRIPT" --result PASS dev-doc 2>&1)
assert_contains "$OUTPUT" "devtest PASS" "PASS 应输出通过"
assert_contains "$OUTPUT" "下一步执行 /test" "全部完成应提示 /test"
assert_file_contains dev-doc/task/task_2026-05-26_1.md "- [x] TASK-T001" "PASS 不应取消勾选"
cleanup

echo "TEST 2: FAIL 取消勾选并写 issue"
setup
OUTPUT=$(bash "$SCRIPT" --result FAIL dev-doc 2>&1)
assert_contains "$OUTPUT" "devtest FAIL" "FAIL 应输出失败"
assert_file_contains dev-doc/task/task_2026-05-26_1.md "- [ ] TASK-T001" "FAIL 应取消勾选"
ISSUE=$(find dev-doc/issue -name "issue_devtest_*.md" | head -1)
assert_file_contains "$ISSUE" "ISSUE-I001" "FAIL 应写 issue"
cleanup

echo "TEST 3: NEEDS_CONTEXT 保持勾选"
setup
OUTPUT=$(bash "$SCRIPT" --result NEEDS_CONTEXT dev-doc 2>&1)
assert_contains "$OUTPUT" "NEEDS_CONTEXT" "应输出 NEEDS_CONTEXT"
assert_file_contains dev-doc/task/task_2026-05-26_1.md "- [x] TASK-T001" "NEEDS_CONTEXT 不应取消勾选"
cleanup

echo "TEST 4: exec_mode 切换"
setup
OUTPUT=$(bash "$SCRIPT" --continuous dev-doc 2>&1)
assert_file_contains dev-doc/STATUS.yaml "exec_mode: continuous" "应写 continuous"
OUTPUT=$(bash "$SCRIPT" --step dev-doc 2>&1)
assert_file_contains dev-doc/STATUS.yaml "exec_mode: step" "应写 step"
cleanup

echo ""
echo "=== devtest minimal 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
