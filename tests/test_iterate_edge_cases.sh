#!/bin/bash
# 测试 iterate.sh 边界情况
# 特别关注 P0 issue 检测逻辑的正确性

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/test_iterate_edge_$$"

PASS=0; FAIL=0; ERRORS=""

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected: '$expected'\n  actual:   '$actual'"
  fi
}

assert_contains() {
  local test_name="$1" expected="$2" actual="$3"
  if echo "$actual" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected to contain: '$expected'\n  actual: '$actual'"
  fi
}

setup_env() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"

  mkdir -p dev-doc/task dev-doc/issue scripts/lib scripts/commands
  cp "$PROJECT_ROOT/scripts/lib/version.sh" scripts/lib/
  cp "$PROJECT_ROOT/scripts/commands/iterate.sh" scripts/commands/

  echo "2.2.0" > VERSION

  cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: TEST
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

  # 全部完成的 task
  cat > dev-doc/task/done_task_v2.2.md << 'EOF'
---
title: TASK
nums: 1
---

- [x] T1：完成
  - level: P0
  - details：已完成
  - Done when：完成
EOF

  git add -A && git commit -q -m "init"
}

cleanup() {
  cd "$PROJECT_ROOT"
  rm -rf "$TMP_DIR"
}

# === BUG验证: P0 已修复（marked [x]）但文件未 rename，应否阻断？ ===
# SPEC 说"无未关闭的 P0 issue"，但 iterate.sh 只检查文件是否含 "severity: P0"，
# 不检查对应条目是否是 [ ] 还是 [x]
test_p0_issue_fixed_but_file_not_renamed() {
  setup_env

  # issue 文件中 P0 已修复（[x]），但有一个 P1 未修复（[ ]）
  # 文件未 rename 为 closed_ 因为还有未关闭条目
  cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
source: test
nums: 2
---

- [x] I1：P0 严重问题（已修复）
  - severity: P0
  - location：test.sh:1
  - description：严重问题
  - reproduce：执行测试
  - fix：已修复

- [ ] I2：P1 普通问题（未修复）
  - severity: P1
  - location：test.sh:10
  - description：普通问题
  - reproduce：执行测试
  - fix：
EOF
  git add -A && git commit -q -m "add mixed issue"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test" 2>&1)
  local code=$?

  # bug 已修复：P0 已修复（[x]）时不应阻断迭代
  # iterate.sh 现在只检查 [ ] 状态的 P0 条目
  assert_eq "P0已修复不应阻断" "0" "$code"
  cleanup
}

# === 正常情况: 只有 closed_ 前缀的 P0 issue 不应阻断 ===
test_closed_p0_issue_no_block() {
  setup_env

  # 文件已 rename 为 closed_
  cat > dev-doc/issue/closed_issue_test_2026-05-24_1.md << 'EOF'
---
source: test
nums: 1
---

- [x] I1：P0 已修复
  - severity: P0
  - location：test.sh:1
  - description：已修复
  - reproduce：测试
  - fix：已修复
EOF
  git add -A && git commit -q -m "add closed issue"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test" 2>&1)
  local code=$?
  assert_eq "closed_P0不阻断" "0" "$code"
  cleanup
}

# === 多文件 issue: 一个文件有 P0 未关闭，另一个无 ===
test_multiple_issue_files_p0() {
  setup_env

  cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
source: test
nums: 1
---

- [ ] I1：P1 问题
  - severity: P1
  - location：test.sh:1
  - description：P1
  - reproduce：测试
  - fix：
EOF

  cat > dev-doc/issue/issue_test_2026-05-24_2.md << 'EOF'
---
source: test
nums: 1
---

- [ ] I1：P0 阻断问题
  - severity: P0
  - location：test.sh:5
  - description：P0
  - reproduce：测试
  - fix：
EOF
  git add -A && git commit -q -m "add issues"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test" 2>&1)
  local code=$?
  assert_eq "有P0未关闭阻断" "1" "$code"
  cleanup
}

# === 空 issue 目录不应阻断 ===
test_empty_issue_dir() {
  setup_env
  # 目录存在但没有文件
  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test" 2>&1)
  local code=$?
  assert_eq "空issue目录不阻断" "0" "$code"
  cleanup
}

# === 无 task 文件时不应阻断 ===
test_no_task_files() {
  setup_env
  rm -f dev-doc/task/done_task_v2.2.md
  git add -A && git commit -q -m "no tasks"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test" 2>&1)
  local code=$?
  # TASK_TOTAL=0 时 condition 是 TASK_TOTAL > 0 && TASK_DONE < TASK_TOTAL，所以不阻断
  assert_eq "无task文件不阻断" "0" "$code"
  cleanup
}

# === 运行所有测试 ===
test_p0_issue_fixed_but_file_not_renamed
test_closed_p0_issue_no_block
test_multiple_issue_files_p0
test_empty_issue_dir
test_no_task_files

# === 报告 ===
echo ""
echo "=========================="
echo "test_iterate_edge_cases.sh 结果"
echo "=========================="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
if [ -n "$ERRORS" ]; then
  echo ""
  echo "失败详情："
  echo -e "$ERRORS"
fi
echo ""
exit $FAIL
