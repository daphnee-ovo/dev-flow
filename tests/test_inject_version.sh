#!/bin/bash
# 测试 inject-context.sh 中版本号注入功能
# 覆盖：T3 - inject-context.sh 版本注入
# 覆盖：T4 - /status 版本展示

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/test_inject_version_$$"

PASS=0; FAIL=0; ERRORS=""

assert_contains() {
  local test_name="$1" expected="$2" actual="$3"
  if echo "$actual" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected to contain: '$expected'\n  actual output:\n$actual"
  fi
}

assert_not_contains() {
  local test_name="$1" unexpected="$2" actual="$3"
  if ! echo "$actual" | grep -qF "$unexpected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected NOT to contain: '$unexpected'\n  actual: '$actual'"
  fi
}

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected: '$expected'\n  actual:   '$actual'"
  fi
}

# 创建测试环境
setup_inject_env() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"

  mkdir -p dev-doc/task dev-doc/issue
  cp "$PROJECT_ROOT/scripts/hooks/inject-context.sh" .

  cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

  echo "2.2.0" > VERSION
  git add -A && git commit -q -m "init"
}

cleanup() {
  cd "$PROJECT_ROOT"
  rm -rf "$TMP_DIR"
}

# === T3: inject-context 输出包含版本号（无 tag） ===
test_inject_version_no_tag() {
  setup_inject_env
  local output
  output=$(bash inject-context.sh 2>&1)
  assert_contains "输出含版本号v2.2.0" "v2.2.0" "$output"
  assert_contains "无tag时显示no-tag" "no-tag" "$output"
  cleanup
}

# === T3: inject-context 输出包含版本号（有 tag） ===
test_inject_version_with_tag() {
  setup_inject_env
  git tag -a "v2.2.0" -m "Release v2.2.0"
  local output
  output=$(bash inject-context.sh 2>&1)
  assert_contains "有tag时显示synced" "synced" "$output"
  assert_contains "有tag时包含版本号" "v2.2.0" "$output"
  cleanup
}

# === T3: inject-context 无 VERSION 文件时不崩溃 ===
test_inject_no_version_file() {
  setup_inject_env
  rm VERSION
  git add -A && git commit -q -m "no version"
  local output
  output=$(bash inject-context.sh 2>&1)
  local code=$?
  assert_eq "无VERSION不崩溃退出码0" "0" "$code"
  # 不应包含版本号输出的部分
  assert_not_contains "无VERSION无版本显示" "v2.2.0" "$output"
  cleanup
}

# === T3: inject-context 输出格式符合 SPEC ===
test_inject_output_format() {
  setup_inject_env
  local output
  output=$(bash inject-context.sh 2>&1)
  # SPEC 格式：[dev-flow <MODE>] v<VER>(<synced|no-tag>) | STAGE: ... | TASK: ... | ISSUE: ...
  if echo "$output" | grep -qE '^\[dev-flow [a-z]+\] v[0-9]+\.[0-9]+\.[0-9]+\((synced|no-tag)\) \| STAGE:'; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: inject输出格式不符合SPEC\n  actual: '$output'"
  fi
  cleanup
}

# === T4: /status 展示版本信息（无 tag） ===
test_status_version_no_tag() {
  setup_inject_env
  cp "$PROJECT_ROOT/scripts/commands/status.sh" .
  local output
  output=$(bash status.sh dev-doc 2>&1)
  assert_contains "status显示版本" "当前版本：v2.2.0" "$output"
  assert_contains "status显示未同步" "未同步" "$output"
  cleanup
}

# === T4: /status 展示版本信息（有 tag） ===
test_status_version_with_tag() {
  setup_inject_env
  git tag -a "v2.2.0" -m "Release v2.2.0"
  cp "$PROJECT_ROOT/scripts/commands/status.sh" .
  local output
  output=$(bash status.sh dev-doc 2>&1)
  assert_contains "status有tag显示已同步" "已同步" "$output"
  cleanup
}

# === T4: /status 无 VERSION 文件时 ===
test_status_no_version_file() {
  setup_inject_env
  rm VERSION
  git add -A && git commit -q -m "no version"
  cp "$PROJECT_ROOT/scripts/commands/status.sh" .
  local output
  output=$(bash status.sh dev-doc 2>&1)
  assert_contains "无VERSION时status提示" "缺少 VERSION" "$output"
  cleanup
}

# === 运行所有测试 ===
test_inject_version_no_tag
test_inject_version_with_tag
test_inject_no_version_file
test_inject_output_format
test_status_version_no_tag
test_status_version_with_tag
test_status_no_version_file

# === 报告 ===
echo ""
echo "=========================="
echo "test_inject_version.sh 结果"
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
