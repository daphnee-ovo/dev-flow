#!/bin/bash
# 测试 scripts/commands/ 下所有命令脚本
# status.sh, check.sh, mode.sh, iterate.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CMD_DIR="$SCRIPT_DIR/scripts/commands"
TMP_DIR="$SCRIPT_DIR/tmp/test_commands_$$"
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

assert_exit_code() {
  local actual="$1" expected="$2" msg="$3"
  if [ "$actual" -eq "$expected" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected exit $expected, got exit $actual"
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: $expected\n    content: $content"
  fi
}

assert_dir_exists() {
  local path="$1" msg="$2"
  if [ -d "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    dir not found: $path"
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

echo "=============================="
echo "  Testing: status.sh"
echo "=============================="

# === status.sh TEST 1: 完整输出格式 ===
echo "STATUS TEST 1: 完整输出格式"
setup
mkdir -p dev-doc/task dev-doc/issue
echo "2.1.0" > VERSION
cat > dev-doc/STATUS.yaml << 'EOF'
name: my-project
phase: DEV
mode: full
updated: 2026-05-15 14:00
started: 2026-05-14 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 功能A
  Done when: pass
  level: P0
- [ ] 功能B
  Done when: pass
  level: P1
EOF
cat > dev-doc/task/done_task_2026-05-14_1.md << 'EOF'
- [x] 旧任务
  Done when: pass
  level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# Changelog

## 2026-05-15
- 14:30 实现功能A
- 13:00 初始化
EOF
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "my-project" "应显示项目名"
assert_contains "$OUTPUT" "DEV" "应显示阶段"
assert_contains "$OUTPUT" "full" "应显示模式"
assert_contains "$OUTPUT" "v2.1.0" "应显示版本号"
assert_contains "$OUTPUT" "2/3" "应显示正确的任务进度"
assert_contains "$OUTPUT" "实现功能A" "应显示最近 CHANGELOG"
assert_contains "$OUTPUT" "继续开发" "DEV 阶段未完成应建议继续开发"

# === status.sh TEST 2: 无 STATUS 时提示 ===
echo "STATUS TEST 2: 无 STATUS 时提示初始化"
setup
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "不存在" "无 STATUS 应提示不存在"

echo ""
echo "=============================="
echo "  Testing: check.sh"
echo "=============================="

# === check.sh TEST 1: 各检查项 ===
echo "CHECK TEST 1: 检查项正常报告"
setup
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-15 14:00
started: 2026-05-15 10:00
EOF
# DEV 阶段无 task → 应报警
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "task/ 目录无任务" "DEV 阶段无 task 应报警"

# === check.sh TEST 2: 所有任务完成但仍在 DEV ===
echo "CHECK TEST 2: 所有任务完成但仍在 DEV 应提示"
setup
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-15 14:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 已完成
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成但阶段仍为 DEV" "应提示升级阶段"

# === check.sh TEST 3: DONE 阶段有 open issue 报警 ===
echo "CHECK TEST 3: DONE 阶段有 open issue"
setup
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-15 14:00
started: 2026-05-15 10:00
EOF
cat > dev-doc/task/done_task_2026-05-15_1.md << 'EOF'
- [x] 完成
  Done when: pass
  level: P0
EOF
cat > dev-doc/issue/issue_test_2026-05-15_1.md << 'EOF'
- [ ] 未修复的 bug
  severity: P0
EOF
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "未关闭 issue" "DONE 阶段有 open issue 应报警"

echo ""
echo "=============================="
echo "  Testing: mode.sh"
echo "=============================="

# === mode.sh TEST 1: 参数校验 ===
echo "MODE TEST 1: 无效参数"
setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" "invalid_mode" 2>&1)
EXIT=$?
assert_exit_code "$EXIT" 1 "无效模式应 exit 1"
assert_contains "$OUTPUT" "无效模式" "应提示无效模式"

# === mode.sh TEST 2: 无参数时显示帮助 ===
echo "MODE TEST 2: 无参数显示帮助"
setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" 2>&1)
EXIT=$?
assert_exit_code "$EXIT" 1 "无参数应 exit 1"
assert_contains "$OUTPUT" "用法" "应显示用法"

# === mode.sh TEST 3: STATUS 不存在时创建 ===
echo "MODE TEST 3: STATUS 不存在时创建"
setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" "fast" "dev-doc" 2>&1)
assert_file_exists "dev-doc/STATUS.yaml" "应创建 STATUS.yaml"
assert_file_contains "dev-doc/STATUS.yaml" "mode: fast" "应设置正确 mode"
assert_file_contains "dev-doc/STATUS.yaml" "phase: TASK" "fast 模式初始 phase 应为 TASK"
assert_contains "$OUTPUT" "新建 STATUS.yaml" "应提示新建"

# === mode.sh TEST 4: STATUS 已存在时更新 ===
echo "MODE TEST 4: STATUS 已存在时更新"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-01 10:00
started: 2026-05-01 10:00
EOF
OUTPUT=$(bash "$CMD_DIR/mode.sh" "quick" "dev-doc" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "mode: quick" "应更新 mode 为 quick"
assert_not_contains "$OUTPUT" "新建" "已存在不应提示新建"

# === mode.sh TEST 5: 各模式初始 phase 正确 ===
echo "MODE TEST 5: full 模式初始 phase=PRD"
setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" "full" "dev-doc" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "phase: PRD" "full → PRD"

setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" "quick" "dev-doc" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "phase: SPEC" "quick → SPEC"

setup
OUTPUT=$(bash "$CMD_DIR/mode.sh" "mvp" "dev-doc" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "phase: SPEC" "mvp → SPEC"

echo ""
echo "=============================="
echo "  Testing: iterate.sh"
echo "=============================="

# === iterate.sh TEST 1: 无参数报错 ===
echo "ITERATE TEST 1: 无 topic 参数"
setup
OUTPUT=$(bash "$CMD_DIR/iterate.sh" 2>&1)
EXIT=$?
assert_exit_code "$EXIT" 1 "无参数应 exit 1"
assert_contains "$OUTPUT" "用法" "应显示用法"

# === iterate.sh TEST 2: 完整归档操作 ===
echo "ITERATE TEST 2: 完整归档操作"
setup
mkdir -p dev-doc/task dev-doc/issue
echo "1.0.0" > VERSION
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-15 14:00
started: 2026-05-14 10:00
EOF
cat > dev-doc/task/done_task_2026-05-14_1.md << 'EOF'
- [x] 已完成任务
EOF
cat > dev-doc/issue/closed_issue_test_2026-05-14_1.md << 'EOF'
- [x] 已关闭 issue
EOF
cat > dev-doc/PRD.md << 'EOF'
# PRD
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# Changelog

## 2026-05-15
- 14:30 完成开发
EOF
git add -A && git commit -m "prepare" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "first-release" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "应输出迭代完成"
assert_dir_exists "dev-doc/archive/v1.0.0-first-release" "应创建归档目录"
assert_file_exists "dev-doc/archive/v1.0.0-first-release/done_task_2026-05-14_1.md" "done_task 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-first-release/issue/closed_issue_test_2026-05-14_1.md" "closed_issue 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-first-release/PRD.md" "PRD 应被归档(copy)"
assert_file_exists "dev-doc/archive/v1.0.0-first-release/SPEC.md" "SPEC 应被归档(copy)"
assert_file_exists "dev-doc/archive/v1.0.0-first-release/CHANGELOG.md" "CHANGELOG 应被归档"
# 原位文件检查
assert_file_not_exists "dev-doc/task/done_task_2026-05-14_1.md" "done_task 原位应被移走"
assert_file_not_exists "dev-doc/issue/closed_issue_test_2026-05-14_1.md" "closed_issue 原位应被移走"
assert_file_exists "dev-doc/PRD.md" "PRD 原位应保留(copy)"
assert_file_exists "dev-doc/CHANGELOG.md" "CHANGELOG 应重建"
assert_file_contains "dev-doc/CHANGELOG.md" "# CHANGELOG" "新 CHANGELOG 应有头部"

# === iterate.sh TEST 3: VERSION bump 和 phase 重置 ===
echo "ITERATE TEST 3: VERSION bump 和 phase 重置"
assert_file_contains "VERSION" "1.1.0" "VERSION 应从 1.0.0 bump 到 1.1.0"
assert_file_contains "dev-doc/STATUS.yaml" "phase: PRD" "full 模式归档后 phase 应重置为 PRD"

# === iterate.sh TEST 4: archive 目录已存在时报错 ===
echo "ITERATE TEST 4: archive 目录已存在时报错"
setup
mkdir -p dev-doc/task dev-doc/issue dev-doc/archive/v1.0.0-dup-topic
echo "1.0.0" > VERSION
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-15 14:00
started: 2026-05-14 10:00
EOF
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "dup-topic" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_exit_code "$EXIT" 1 "归档目录已存在应 exit 1"
assert_contains "$OUTPUT" "归档目录已存在" "应提示归档目录已存在"

# === 汇总 ===
teardown
echo ""
echo "=== commands 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
