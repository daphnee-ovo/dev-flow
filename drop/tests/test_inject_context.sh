#!/bin/bash
# 测试 scripts/hooks/inject-context.sh
# 验证项目上下文注入逻辑

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$SCRIPT_DIR/scripts/hooks/inject-context.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_inject_context_$$"
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

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if ! echo "$output" | grep -qF "$unexpected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected NOT to contain: $unexpected\n    got: $(echo "$output" | head -5)"
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

assert_exit_zero() {
  local code="$1" msg="$2"
  if [ "$code" -eq 0 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected exit 0, got exit $code"
  fi
}

# === TEST 1: 无 dev-doc 时正常退出 ===
echo "TEST 1: 无 dev-doc 时正常退出"
setup
OUTPUT=$(bash "$HOOK" 2>&1)
EXIT_CODE=$?
assert_exit_zero "$EXIT_CODE" "无 dev-doc 应 exit 0"
assert_empty "$OUTPUT" "无 dev-doc 应无输出"

# === TEST 2: dev-doc 存在但无 STATUS.yaml ===
echo "TEST 2: dev-doc 存在但无 STATUS.yaml"
setup
mkdir -p dev-doc
OUTPUT=$(bash "$HOOK" 2>&1)
EXIT_CODE=$?
assert_exit_zero "$EXIT_CODE" "无 STATUS.yaml 应 exit 0"
assert_empty "$OUTPUT" "无 STATUS.yaml 应无输出"

# === TEST 3: task/ 目录为空时正常处理 ===
echo "TEST 3: task/ 目录为空时正常处理"
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
assert_exit_zero "$EXIT_CODE" "task/ 为空应 exit 0"
assert_contains "$OUTPUT" "TASK: 0/0" "task/ 为空时应显示 0/0"
# DEV 阶段无活跃 task 且无 issue → BLOCKED
assert_contains "$OUTPUT" "BLOCKED" "DEV 阶段无 task 无 issue 应输出 BLOCKED"

# === TEST 4: done_task_* 文件正确计入总数 ===
echo "TEST 4: done_task_* 文件计入总数"
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
# 一个活跃 task（1 done, 1 undone）
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [x] 完成功能A
  Done when: 测试通过
  level: P0
- [ ] 完成功能B
  Done when: 测试通过
  level: P0
EOF
# 一个已完成 task（2 done）
cat > dev-doc/task/done_task_2026-05-14_1.md << 'EOF'
- [x] 完成功能C
  Done when: 测试通过
  level: P0
- [x] 完成功能D
  Done when: 测试通过
  level: P1
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_contains "$OUTPUT" "TASK: 3/4" "done_task 应计入总数: 3/4"

# === TEST 5: issue/task 互斥展示逻辑 ===
echo "TEST 5: issue/task 互斥展示 - 有 issue 时展示 issue"
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
- [ ] 完成功能A
  Done when: 测试通过
  level: P0
EOF
cat > dev-doc/issue/issue_test_2026-05-15_1.md << 'EOF'
- [ ] 修复 bug1
  severity: P0
- [x] 修复 bug2
  severity: P1
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_contains "$OUTPUT" "ISSUE" "有 issue 时应展示 ISSUE LIST"
assert_not_contains "$OUTPUT" "TASK LIST" "有 issue 时不应展示 TASK LIST"

# === TEST 6: BUG#1 — grep -c || echo 0 算术错误 ===
echo "TEST 6: BUG#1 — 当 task 文件缺少某 level 时触发算术错误"
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
# 只有 P0，无 P1/P2 → grep -c "level: P1" 输出 "0" exit 1 → || echo 0 追加第二个 "0"
# 导致 $((P1_TOTAL + 0\n0)) 算术错误
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [ ] 完成功能A
  Done when: 测试通过
  level: P0
- [x] 完成功能B
  Done when: 测试通过
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
if echo "$OUTPUT" | grep -q "syntax error"; then
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: [BUG#1] inject-context.sh lines 37-39: grep -c || echo 0 产生 '0\\n0' 算术错误\n    当 task 文件不含某个 level(P1/P2)时，grep -c 输出 '0'(exit 1) + echo 0 产生 '0\\n0'\n    修复方案：将 '|| echo 0' 改为 '|| true'（grep -c 已输出 0）"
else
  PASS=$((PASS + 1))
fi

# === TEST 6b: BUG#2 — awk getline 只读一行，漏过 Done when 找不到 level ===
echo "TEST 6b: BUG#2 — awk getline 跳过 Done when 行导致 level 匹配失败"
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
# 文件含所有 level 避免 BUG#1，但 Done when 在 level 前面
# awk 只做一次 getline 只能读到 Done when，读不到 level
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [ ] 完成功能A
  Done when: 测试通过
  level: P0
- [x] 完成功能B
  Done when: 测试通过
  level: P0
- [ ] P1任务
  Done when: pass
  level: P1
- [ ] P2任务
  Done when: pass
  level: P2
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
if echo "$OUTPUT" | grep -qF "TASK LIST"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: [BUG#2] inject-context.sh lines 128-131: awk getline 只读下一行\n    文件格式为: '- [ ] name / Done when: ... / level: Px'\n    但 awk 在 checkbox 行后只做一次 getline，读到 'Done when' 而非 'level'\n    修复方案：在 awk 中多做一次 getline，或改用多行 grep/sed 方案"
fi

# === TEST 7: 优先级分层逻辑 — 使用 level 紧跟 checkbox 的格式（绕过 BUG#2） ===
echo "TEST 7: 优先级分层 - level 紧跟 checkbox 行时正常工作"
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
# 将 level 放在 checkbox 后第一行（无 Done when 间隔）以绕过 BUG#2
cat > dev-doc/task/task_2026-05-15_1.md << 'EOF'
- [ ] P0任务未完成
  level: P0
- [ ] P1任务未完成
  level: P1
- [ ] P2任务
  level: P2
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
# 注意: 同样会触发 BUG#1（grep -c "^- \[x\]" 对全是 [ ] 的文件返回 0 exit 1）
# 但 DONE + 0\n0 错误不影响后续流程（只是额外 stderr），awk 部分应正常工作
if echo "$OUTPUT" | grep -qF "[P0 TASK LIST]"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 即使 level 紧跟 checkbox，P0 TASK LIST 仍未展示\n    output: $(echo "$OUTPUT" | head -5)"
fi

# === TEST 8: 优先级分层 - P0 全完成展示 P1（level 紧跟 checkbox） ===
echo "TEST 8: 优先级分层 - P0 全完成后展示 P1"
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
- [x] P0任务已完成
  level: P0
- [ ] P1任务未完成
  level: P1
- [ ] P2任务
  level: P2
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
if echo "$OUTPUT" | grep -qF "[P1 TASK LIST]"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: P0 全完成后应展示 P1 TASK LIST\n    output: $(echo "$OUTPUT" | head -5)"
fi

# === TEST 9: BLOCKED 阻断（DEV 阶段无活跃 task 且无 open issue） ===
echo "TEST 9: BLOCKED 阻断"
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
# 只有 done_ task，没有活跃 task
cat > dev-doc/task/done_task_2026-05-15_1.md << 'EOF'
- [x] 已完成
  Done when: pass
  level: P0
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成" "只有 done_task 应引导 /test"

# === TEST 10: CHANGELOG 最近条目注入 ===
echo "TEST 10: CHANGELOG 最近条目注入"
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
- [ ] 做点什么
  Done when: pass
  level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# Changelog

## 2026-05-15
- 14:30 实现了核心功能
- 13:00 项目初始化
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_contains "$OUTPUT" "[LAST] - 14:30 实现了核心功能" "应展示最近 CHANGELOG 条目"

# === TEST 11: 多工程模式（分支 DOC_ROOT） ===
echo "TEST 11: 多工程模式"
setup
git checkout -b feature-test -q
mkdir -p dev-doc/feature-test/task
cat > dev-doc/feature-test/STATUS.yaml << 'EOF'
name: test-project
phase: SPEC
mode: quick
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$HOOK" 2>&1)
assert_contains "$OUTPUT" "STAGE: SPEC" "多工程模式应读取分支对应的 STATUS"
assert_contains "$OUTPUT" "quick" "多工程模式应读取分支对应的 mode"

# === 汇总 ===
teardown
echo ""
echo "=== inject-context.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
