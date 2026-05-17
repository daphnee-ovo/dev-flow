#!/bin/bash
# 测试 scripts/init/scan-project.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$SCRIPT_DIR/scripts/init/scan-project.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_scan_project_$$"
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: $expected\n    got: $(echo "$output" | head -15)"
  fi
}

assert_matches() {
  local output="$1" pattern="$2" msg="$3"
  if echo "$output" | grep -qE "$pattern"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to match: $pattern\n    got: $(echo "$output" | head -15)"
  fi
}

# === TEST 1: task_summary 输出 ===
echo "TEST 1: task_summary 输出"
setup
mkdir -p dev-doc/task
cat > "dev-doc/task/task_2026-05-15_1.md" << 'EOF'
- [ ] 活跃任务
EOF
cat > "dev-doc/task/task_2026-05-15_2.md" << 'EOF'
- [ ] 另一个活跃任务
EOF
cat > "dev-doc/task/done_task_2026-05-14_1.md" << 'EOF'
- [x] 已完成
EOF
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "task_summary: active=2 done=1" "应正确统计 active 和 done task"

# === TEST 2: issue_summary 输出 ===
echo "TEST 2: issue_summary 输出"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
- [ ] bug
EOF
cat > "dev-doc/issue/issue_test_2026-05-15_2.md" << 'EOF'
- [ ] bug2
EOF
cat > "dev-doc/issue/closed_issue_test_2026-05-14_1.md" << 'EOF'
- [x] 已修复
EOF
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "issue_summary: open=2 closed=1" "应正确统计 open 和 closed issue"

# === TEST 3: 无 dev-doc 时输出 none ===
echo "TEST 3: 无 dev-doc 时输出 none"
setup
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "none" "无 dev-doc 时应输出 none"

# === TEST 4: 项目名检测 ===
echo "TEST 4: 项目名检测"
setup
cat > package.json << 'EOF'
{
  "name": "my-awesome-app",
  "version": "1.0.0"
}
EOF
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "name: my-awesome-app" "应从 package.json 读取项目名"

# === TEST 5: 技术栈检测 ===
echo "TEST 5: 技术栈检测"
setup
echo '{}' > package.json
echo '{}' > tsconfig.json
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "node" "应检测到 node"
assert_contains "$OUTPUT" "typescript" "应检测到 typescript"

# === TEST 6: 基本结构输出 ===
echo "TEST 6: 基本结构输出完整"
setup
OUTPUT=$(bash "$SCRIPT" 2>&1)
assert_contains "$OUTPUT" "=== PROJECT SCAN ===" "应有 header"
assert_contains "$OUTPUT" "name:" "应有 name"
assert_contains "$OUTPUT" "stack:" "应有 stack"
assert_contains "$OUTPUT" "git:" "应有 git 信息"

# === 汇总 ===
teardown
echo ""
echo "=== scan-project.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
