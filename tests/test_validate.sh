#!/bin/bash
# 测试 scripts/init/validate.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$SCRIPT_DIR/scripts/init/validate.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_validate_$$"
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

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if ! echo "$output" | grep -qF "$unexpected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected NOT to contain: $unexpected"
  fi
}

assert_dir_exists() {
  local path="$1" msg="$2"
  if [ -d "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    directory not found: $path"
  fi
}

assert_dir_not_exists() {
  local path="$1" msg="$2"
  if [ ! -d "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    directory should not exist: $path"
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

# === TEST 1: 创建 task/ 而非 session/memory ===
echo "TEST 1: 创建 task/ 目录而非 session/memory"
setup
mkdir -p dev-doc
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_dir_exists "dev-doc/task" "应创建 task/ 目录"
assert_dir_not_exists "dev-doc/session" "不应创建 session/ 目录"
assert_dir_not_exists "dev-doc/memory" "不应创建 memory/ 目录"
# 也应创建 issue/ 和 archive/
assert_dir_exists "dev-doc/issue" "应创建 issue/ 目录"
assert_dir_exists "dev-doc/archive" "应创建 archive/ 目录"

# === TEST 2: phase 枚举不含 MVP ===
echo "TEST 2: phase 枚举不含 MVP"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: MVP
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "status_invalid_phase:MVP" "MVP 应被识别为无效 phase"

# === TEST 3: 合法 phase 不报错 ===
echo "TEST 3: 合法 phase 不报错"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_not_contains "$OUTPUT" "status_invalid_phase" "DEV 应为合法 phase"

# === TEST 4: task 文件命名校验 ===
echo "TEST 4: task 文件命名校验"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
# 正确命名
cat > "dev-doc/task/task_2026-05-15_1.md" << 'EOF'
- [ ] 任务
  Done when: pass
  level: P0
EOF
# 错误命名（无日期）
cat > "dev-doc/task/task_feature_login.md" << 'EOF'
- [ ] 任务
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "task_bad_name:task_feature_login.md" "不规范命名应报告"
assert_not_contains "$OUTPUT" "task_bad_name:task_2026-05-15_1.md" "规范命名不应报告"

# === TEST 5: issue checkbox/prefix 一致性检查 ===
echo "TEST 5: issue checkbox/prefix 一致性检查"
setup
mkdir -p dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
# issue 全部勾选但无 closed_ 前缀
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
---
- [x] bug 已修复
  severity: P0
EOF
# closed_ 前缀但有未勾选
cat > "dev-doc/issue/closed_issue_test_2026-05-15_2.md" << 'EOF'
---
source: test
---
- [ ] bug 未修复
  severity: P1
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_should_be_closed:issue_test_2026-05-15_1.md" "全勾选无 closed_ 前缀应报告"
assert_contains "$OUTPUT" "issue_closed_but_open_items:closed_issue_test_2026-05-15_2.md" "有 closed_ 但未全勾选应报告"

# === TEST 6: CHANGELOG 自动创建 ===
echo "TEST 6: CHANGELOG 不存在时自动创建"
setup
mkdir -p dev-doc
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_file_exists "dev-doc/CHANGELOG.md" "应自动创建 CHANGELOG.md"
assert_contains "$OUTPUT" "created_changelog" "应报告自动创建 CHANGELOG"

# === TEST 7: 已有 CHANGELOG 不覆盖 ===
echo "TEST 7: 已有 CHANGELOG 不覆盖"
setup
mkdir -p dev-doc
echo "# 已有内容" > dev-doc/CHANGELOG.md
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
CONTENT=$(cat dev-doc/CHANGELOG.md)
if echo "$CONTENT" | grep -qF "已有内容"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 已有 CHANGELOG 不应被覆盖"
fi

# === TEST 8: tmp/ 目录创建和 .gitignore ===
echo "TEST 8: tmp/ 和 .gitignore"
setup
mkdir -p dev-doc
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_dir_exists "tmp" "应创建 tmp/ 目录"
assert_file_exists ".gitignore" "应创建 .gitignore"
if grep -qF "tmp/" .gitignore; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: .gitignore 应含 tmp/"
fi

# === 汇总 ===
teardown
echo ""
echo "=== validate.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
