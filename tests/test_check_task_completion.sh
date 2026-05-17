#!/bin/bash
# 测试 scripts/hooks/check-task-completion.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$SCRIPT_DIR/scripts/hooks/check-task-completion.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_check_task_$$"
PASS=0; FAIL=0; ERRORS=""

setup() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q .
  git commit --allow-empty -m "init" -q
}

teardown() {
  cd "$SCRIPT_DIR"
  rm -rf "$TMP_DIR"
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected\n    got: $(echo "$output" | head -5)"
  fi
}

assert_file_exists() {
  local path="$1" msg="$2"
  if [ -f "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    file not found: $path"
  fi
}

assert_file_not_exists() {
  local path="$1" msg="$2"
  if [ ! -f "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    file should not exist: $path"
  fi
}

assert_empty() {
  local output="$1" msg="$2"
  if [ -z "$output" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected empty output\n    got: $output"
  fi
}

# === TEST 1: 非 DEV 阶段不触发 ===
echo "TEST 1: 非 DEV 阶段不触发"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: SPEC
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 已完成
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_empty "$OUTPUT" "非 DEV 阶段不应输出任何内容"
# 确认文件未被重命名
assert_file_exists "dev-doc/task/task_2026-05-15_1.md" "非 DEV 阶段不应重命名文件"

# === TEST 2: task 文件全部勾选时自动重命名为 done_ 前缀 ===
echo "TEST 2: task 全部勾选时自动重命名为 done_"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 功能A完成
  Done when: 测试通过
  level: P0
- [x] 功能B完成
  Done when: 测试通过
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_file_exists "dev-doc/task/done_task_2026-05-15_1.md" "全部勾选应重命名为 done_"
assert_file_not_exists "dev-doc/task/task_2026-05-15_1.md" "原文件应被移走"
assert_contains "$OUTPUT" "done_task_2026-05-15_1.md" "应输出重命名信息"

# === TEST 3: issue 文件全部勾选时自动重命名为 closed_ 前缀 ===
echo "TEST 3: issue 全部勾选时自动重命名为 closed_"
setup
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [ ] 还有活跃 task
  Done when: pass
  level: P0
EOF
cat > dev-doc/issue/issue_test_2026-05-15_1.md << 'EOF'
- [x] bug1 修复
  severity: P0
- [x] bug2 修复
  severity: P1
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_file_exists "dev-doc/issue/closed_issue_test_2026-05-15_1.md" "全部勾选应重命名为 closed_"
assert_file_not_exists "dev-doc/issue/issue_test_2026-05-15_1.md" "原 issue 应被移走"
assert_contains "$OUTPUT" "closed_issue_test_2026-05-15_1.md" "应输出关闭信息"

# === TEST 4: 不重复重命名已有 done_/closed_ 文件 ===
echo "TEST 4: 不重复重命名已有 done_ 文件"
setup
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
# 模拟已有 done_ 文件的情况：task 全部完成但 done_ 已存在
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 功能完成
  Done when: pass
  level: P0
EOF
cat > dev-doc/task/done_task_2026-05-15_1.md << 'EOF'
- [x] 旧的已完成任务
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
# 原文件应保留（因为 done_ 已存在，不会覆盖）
assert_file_exists "dev-doc/task/task_2026-05-15_1.md" "done_ 已存在时不应重命名"
assert_file_exists "dev-doc/task/done_task_2026-05-15_1.md" "已有 done_ 应保留"

# === TEST 5: task/ 为空时正常退出 ===
echo "TEST 5: task/ 为空时正常退出"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
EXIT_CODE=$?
assert_empty "$OUTPUT" "task/ 为空应无输出"

# === TEST 6: 部分完成不重命名 ===
echo "TEST 6: 部分完成不重命名"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 功能A
  Done when: pass
  level: P0
- [ ] 功能B 未完成
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_file_exists "dev-doc/task/task_2026-05-15_1.md" "部分完成不应重命名"
assert_file_not_exists "dev-doc/task/done_task_2026-05-15_1.md" "部分完成不应生成 done_"

# === 汇总 ===
teardown
echo ""
echo "=== check-task-completion.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
