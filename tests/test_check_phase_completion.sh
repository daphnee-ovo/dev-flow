#!/bin/bash
# 测试 scripts/hooks/check-phase-completion.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$SCRIPT_DIR/scripts/hooks/check-phase-completion.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_check_phase_$$"
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

assert_empty() {
  local output="$1" msg="$2"
  if [ -z "$output" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected empty, got: $output"
  fi
}

# === TEST 1: 非 dev-doc/ 路径文件不触发 ===
echo "TEST 1: 非 dev-doc/ 路径不触发"
setup
mkdir -p dev-doc
OUTPUT=$(bash "$HOOK" "src/main.py" 2>&1)
assert_empty "$OUTPUT" "非 dev-doc 路径应无输出"

# === TEST 2: task/ 路径下完全缺少 Done when 时报告问题 ===
echo "TEST 2: task/ 完全无 Done when 时报告"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: TASK
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
# Task Batch

- [ ] 实现登录功能
  level: P0
- [ ] 实现注册功能
  level: P0
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/task/task_2026-05-15_1.md" 2>&1)
assert_contains "$OUTPUT" "Done when" "task 完全缺少 Done when 应报告问题"

# === TEST 3: PRD.md 检查 ===
echo "TEST 3: PRD.md 完成标准检查"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: PRD
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/PRD.md << 'EOF'
# PRD

## 1. 背景
blah

## 3. 功能需求
blah
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/PRD.md" 2>&1)
assert_contains "$OUTPUT" "目标与非目标" "PRD 缺少目标与非目标应报告"
assert_contains "$OUTPUT" "成功指标" "PRD 缺少成功指标应报告"

# === TEST 4: SPEC.md 检查 ===
echo "TEST 4: SPEC.md 完成标准检查"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: SPEC
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC

## 1. 概述
blah
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/SPEC.md" 2>&1)
assert_contains "$OUTPUT" "架构设计" "SPEC 缺少架构设计应报告"
assert_contains "$OUTPUT" "技术选型" "SPEC 缺少技术选型应报告"
assert_contains "$OUTPUT" "数据模型" "SPEC 缺少数据模型应报告"

# === TEST 5: 多工程模式 task/ 路径适配 ===
echo "TEST 5: 多工程模式下 task/ 路径适配"
setup
mkdir -p dev-doc/feature-x/task
cat > dev-doc/feature-x/STATUS.yaml << 'EOF'
name: test
phase: TASK
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/feature-x/task/task_2026-05-15_1.md << 'EOF'
- [ ] 实现功能
  Done when: 通过测试
  level: P0
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/feature-x/task/task_2026-05-15_1.md" 2>&1)
# 应正确识别多工程模式，不应报错
assert_empty "$OUTPUT" "有 Done when 的 task 不应报问题"

# === 汇总 ===
teardown
echo ""
echo "=== check-phase-completion.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
