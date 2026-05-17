#!/bin/bash
# 测试 scripts/hooks/update-status.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$SCRIPT_DIR/scripts/hooks/update-status.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_update_status_$$"
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

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content=""
    [ -f "$path" ] && content=$(cat "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected\n    content: $content"
  fi
}

assert_file_not_contains() {
  local path="$1" unexpected="$2" msg="$3"
  if [ -f "$path" ] && ! grep -qF "$unexpected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected NOT to contain: $unexpected"
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

# === TEST 1: 非 dev-doc 路径不触发 ===
echo "TEST 1: 非 dev-doc 路径不触发"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "src/main.py" 2>&1)
assert_empty "$OUTPUT" "非 dev-doc 路径应无输出"
assert_file_contains "dev-doc/STATUS.yaml" "updated: 2026-05-01 10:00" "非 dev-doc 路径不应更新时间戳"

# === TEST 2: 跳过 CHANGELOG.md 变更 ===
echo "TEST 2: 跳过 CHANGELOG.md 变更"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/CHANGELOG.md" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "updated: 2026-05-01 10:00" "CHANGELOG 变更不应更新时间戳"

# === TEST 3: task/ 变更触发更新 ===
echo "TEST 3: task/ 变更触发更新"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/task/task_2026-05-15_1.md" 2>&1)
assert_file_not_contains "dev-doc/STATUS.yaml" "updated: 2026-05-01 10:00" "task/ 变更应更新时间戳"
# 验证新时间戳格式
TODAY=$(date +%Y-%m-%d)
assert_file_contains "dev-doc/STATUS.yaml" "updated: $TODAY" "时间戳应更新为今天"

# === TEST 4: 跳过 STATUS.yaml 自身 ===
echo "TEST 4: 跳过 STATUS.yaml 自身变更"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/STATUS.yaml" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "updated: 2026-05-01 10:00" "STATUS.yaml 自身变更不应触发更新"

# === TEST 5: PRD.md 变更触发更新 ===
echo "TEST 5: PRD.md 变更触发更新"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: PRD
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/PRD.md" 2>&1)
assert_file_not_contains "dev-doc/STATUS.yaml" "updated: 2026-05-01 10:00" "PRD.md 变更应更新时间戳"

# === TEST 6: 多工程模式 ===
echo "TEST 6: 多工程模式"
setup
mkdir -p dev-doc/feature-x/task
cat > dev-doc/feature-x/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$HOOK" "dev-doc/feature-x/task/task_2026-05-15_1.md" 2>&1)
assert_file_not_contains "dev-doc/feature-x/STATUS.yaml" "updated: 2026-05-01 10:00" "多工程模式 task 变更应更新时间戳"

# === 汇总 ===
teardown
echo ""
echo "=== update-status.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
