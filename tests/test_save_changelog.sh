#!/bin/bash
# 测试 scripts/hooks/save-changelog.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$SCRIPT_DIR/scripts/hooks/save-changelog.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_save_changelog_$$"
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected\n    got: $(echo "$output" | head -10)"
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

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content=""
    [ -f "$path" ] && content=$(cat "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    file: $path\n    expected to contain: $expected\n    content: $content"
  fi
}

assert_file_matches() {
  local path="$1" pattern="$2" msg="$3"
  if [ -f "$path" ] && grep -qE "$pattern" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content=""
    [ -f "$path" ] && content=$(cat "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    file: $path\n    expected to match: $pattern\n    content: $content"
  fi
}

# === TEST 1: CHANGELOG.md 不存在时自动创建 ===
echo "TEST 1: CHANGELOG.md 不存在时自动创建"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_file_exists "dev-doc/CHANGELOG.md" "应自动创建 CHANGELOG.md"
assert_file_contains "dev-doc/CHANGELOG.md" "# Changelog" "应含 # Changelog 头部"

# === TEST 2: 正确插入日期段 ===
echo "TEST 2: 正确插入日期段"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
TODAY=$(date +%Y-%m-%d)
assert_file_contains "dev-doc/CHANGELOG.md" "## $TODAY" "应插入当天日期段"

# === TEST 3: 追加格式为 - HH:MM <topic> ===
echo "TEST 3: 追加格式正确"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
git commit --allow-empty -m "实现了登录功能" -q
OUTPUT=$(bash "$HOOK" 2>&1)
# 验证格式：- HH:MM <topic>
assert_file_matches "dev-doc/CHANGELOG.md" "^- [0-9]{2}:[0-9]{2} " "应追加 - HH:MM 格式"
assert_file_contains "dev-doc/CHANGELOG.md" "实现了登录功能" "应包含 git commit message 作为 topic"

# === TEST 4: git commit 存在时使用 commit message 作为 topic ===
echo "TEST 4: 有 git commit 时使用 commit message"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: SPEC
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
# git init 后有 "init" commit
OUTPUT=$(bash "$HOOK" 2>&1)
# 有 git commit 时应取 commit message "init"
assert_file_matches "dev-doc/CHANGELOG.md" "^- [0-9]{2}:[0-9]{2} init" "有 git commit 时应使用 commit message 作为 topic"

# === TEST 5: 重复运行同一天不重复创建日期段 ===
echo "TEST 5: 重复运行不重复创建日期段"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
bash "$HOOK" > /dev/null 2>&1
bash "$HOOK" > /dev/null 2>&1
TODAY=$(date +%Y-%m-%d)
DATE_COUNT=$(grep -c "^## $TODAY" dev-doc/CHANGELOG.md)
if [ "$DATE_COUNT" -eq 1 ]; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 重复运行应只有一个日期段\n    count: $DATE_COUNT"
fi

# === TEST 6: 无 dev-doc 时正常退出 ===
echo "TEST 6: 无 dev-doc 时正常退出"
setup
OUTPUT=$(bash "$HOOK" 2>&1)
EXIT_CODE=$?
if [ "$EXIT_CODE" -eq 0 ] && [ -z "$OUTPUT" ]; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 无 dev-doc 应静默退出"
fi

# === 汇总 ===
teardown
echo ""
echo "=== save-changelog.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
