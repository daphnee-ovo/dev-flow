#!/bin/bash
# 测试 Audit Mode 自动审计模式
# 覆盖：SPEC-AC-001 ~ SPEC-AC-004
# 验证：is_audit_mode / enter_audit_mode / mode.sh 拒绝 / post-write 触发 / iterate 恢复 / inject-context 输出

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
export PATH="$PROJECT_ROOT/tests/bin:$PATH"
TMP_DIR="$PROJECT_ROOT/tmp/test_audit_mode_$$"

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

assert_not_contains() {
  local test_name="$1" unexpected="$2" actual="$3"
  if ! echo "$actual" | grep -qF "$unexpected"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected NOT to contain: '$unexpected'\n  actual: '$actual'"
  fi
}

assert_exit_code() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected exit code: $expected\n  actual exit code:   $actual"
  fi
}

# =============================================
# 基础环境设置
# =============================================
setup_basic_env() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"

  mkdir -p dev-doc/task dev-doc/issue scripts/lib scripts/commands scripts/hooks
  cp "$PROJECT_ROOT/scripts/lib/common.sh" scripts/lib/
  cp "$PROJECT_ROOT/scripts/lib/version.sh" scripts/lib/
  cp "$PROJECT_ROOT/scripts/commands/mode.sh" scripts/commands/
  cp "$PROJECT_ROOT/scripts/commands/iterate.sh" scripts/commands/
  cp "$PROJECT_ROOT/scripts/hooks/post-write.sh" scripts/hooks/
  cp "$PROJECT_ROOT/scripts/hooks/inject-context.sh" scripts/hooks/

  echo "2.0.0" > VERSION
  mkdir -p tmp

  git add -A && git commit -q -m "init"
}

create_status() {
  local mode="$1" phase="$2"
  cat > dev-doc/STATUS.yaml << EOF
name: test-project
phase: $phase
mode: $mode
updated: 2026-05-26 10:00
started: 2026-05-26 10:00
EOF
}

# =============================================
# TEST GROUP 1: is_audit_mode 函数行为
# =============================================
test_is_audit_mode_positive() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "audit/quick"
  assert_exit_code "is_audit_mode('audit/quick') 应返回 0" "0" "$?"
}

test_is_audit_mode_positive_full() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "audit/full"
  assert_exit_code "is_audit_mode('audit/full') 应返回 0" "0" "$?"
}

test_is_audit_mode_negative_quick() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "quick"
  assert_exit_code "is_audit_mode('quick') 应返回 1" "1" "$?"
}

test_is_audit_mode_negative_full() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "full"
  assert_exit_code "is_audit_mode('full') 应返回 1" "1" "$?"
}

test_is_audit_mode_negative_fast() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "fast"
  assert_exit_code "is_audit_mode('fast') 应返回 1" "1" "$?"
}

test_is_audit_mode_negative_empty() {
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode ""
  assert_exit_code "is_audit_mode('') 应返回 1" "1" "$?"
}

test_is_audit_mode_edge_audit_only() {
  # "audit" 不含 "/" 不应匹配 "audit/*"
  cd "$TMP_DIR"
  source scripts/lib/common.sh
  is_audit_mode "audit"
  assert_exit_code "is_audit_mode('audit') 无斜杠应返回 1" "1" "$?"
}

# =============================================
# TEST GROUP 2: enter_audit_mode 函数行为
# =============================================
test_enter_audit_mode_basic() {
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  source scripts/lib/common.sh
  enter_audit_mode "dev-doc/STATUS.yaml" > /dev/null

  local mode
  mode=$(devflow_yaml_get "dev-doc/STATUS.yaml" "mode")
  assert_eq "enter_audit_mode 将 mode 设为 audit/quick" "audit/quick" "$mode"

  local phase
  phase=$(devflow_yaml_get "dev-doc/STATUS.yaml" "phase")
  assert_eq "enter_audit_mode 将 phase 设为 DEV" "DEV" "$phase"
}

test_enter_audit_mode_from_full() {
  cd "$TMP_DIR"
  create_status "full" "PRD"
  source scripts/lib/common.sh
  enter_audit_mode "dev-doc/STATUS.yaml" > /dev/null

  local mode
  mode=$(devflow_yaml_get "dev-doc/STATUS.yaml" "mode")
  assert_eq "enter_audit_mode(full) 将 mode 设为 audit/full" "audit/full" "$mode"
}

test_enter_audit_mode_already_audit_returns_1() {
  cd "$TMP_DIR"
  create_status "audit/quick" "DEV"
  source scripts/lib/common.sh
  enter_audit_mode "dev-doc/STATUS.yaml" > /dev/null
  local rc=$?
  assert_exit_code "已是 audit 模式时 enter_audit_mode 返回 1" "1" "$rc"

  # 确认 mode 没变
  local mode
  mode=$(devflow_yaml_get "dev-doc/STATUS.yaml" "mode")
  assert_eq "已是 audit 模式时 mode 不变" "audit/quick" "$mode"
}

test_enter_audit_mode_no_nested_audit() {
  # 防止 audit/audit/quick 嵌套
  cd "$TMP_DIR"
  create_status "audit/quick" "DEV"
  source scripts/lib/common.sh
  enter_audit_mode "dev-doc/STATUS.yaml" > /dev/null

  local mode
  mode=$(devflow_yaml_get "dev-doc/STATUS.yaml" "mode")
  assert_not_contains "不应出现 audit/audit 嵌套" "audit/audit" "$mode"
}

# =============================================
# TEST GROUP 3: mode.sh 拒绝 audit 输入
# =============================================
test_mode_rejects_audit() {
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/commands/mode.sh audit dev-doc 2>&1)
  local rc=$?
  assert_exit_code "mode.sh audit 退出码为 1" "1" "$rc"
  assert_contains "mode.sh audit 输出包含'不支持手动设置'" "不支持手动设置" "$output"
}

test_mode_rejects_audit_slash_quick() {
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/commands/mode.sh audit/quick dev-doc 2>&1)
  local rc=$?
  assert_exit_code "mode.sh audit/quick 退出码为 1" "1" "$rc"
  assert_contains "mode.sh audit/quick 输出包含'不支持手动设置'" "不支持手动设置" "$output"
}

test_mode_accepts_quick() {
  cd "$TMP_DIR"
  create_status "full" "PRD"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/commands/mode.sh quick dev-doc 2>&1)
  local rc=$?
  assert_exit_code "mode.sh quick 正常执行" "0" "$rc"
  assert_contains "mode.sh quick 输出确认设置" "模式已设置" "$output"
}

test_mode_accepts_fast() {
  cd "$TMP_DIR"
  create_status "full" "PRD"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/commands/mode.sh fast dev-doc 2>&1)
  local rc=$?
  assert_exit_code "mode.sh fast 正常执行" "0" "$rc"
  assert_contains "mode.sh fast 输出确认设置" "模式已设置" "$output"
}

test_mode_rejects_audit_variant() {
  # 测试 "auditing" 等 audit 前缀的变体
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/commands/mode.sh auditing dev-doc 2>&1)
  local rc=$?
  # "auditing" 以 audit 开头，应该被拒绝
  assert_exit_code "mode.sh auditing 应被拒绝" "1" "$rc"
}

# =============================================
# TEST GROUP 4: post-write.sh 触发条件
# =============================================
test_post_write_triggers_audit_on_issue_create() {
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  # 模拟创建 issue 文件
  mkdir -p dev-doc/issue
  echo "- [ ] Bug: something broken" > dev-doc/issue/issue_test_2026-05-26_1.md

  # 运行 post-write hook
  export TOOL_INPUT_FILE_PATH="dev-doc/issue/issue_test_2026-05-26_1.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode phase
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')

  assert_eq "post-write issue 创建后 mode 变为 audit/quick" "audit/quick" "$mode"
  assert_eq "post-write issue 创建后 phase 变为 DEV" "DEV" "$phase"
}

test_post_write_no_trigger_when_already_audit() {
  cd "$TMP_DIR"
  create_status "audit/quick" "DEV"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  mkdir -p dev-doc/issue
  echo "- [ ] Another bug" > dev-doc/issue/issue_test_2026-05-26_2.md

  export TOOL_INPUT_FILE_PATH="dev-doc/issue/issue_test_2026-05-26_2.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "已是 audit 模式时不重复触发" "audit/quick" "$mode"
}

test_post_write_no_trigger_when_phase_DEV() {
  cd "$TMP_DIR"
  create_status "quick" "DEV"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  mkdir -p dev-doc/issue
  echo "- [ ] DEV phase bug" > dev-doc/issue/issue_test_2026-05-26_3.md

  export TOOL_INPUT_FILE_PATH="dev-doc/issue/issue_test_2026-05-26_3.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "DEV 阶段创建 issue 不触发 audit" "quick" "$mode"
}

test_post_write_no_trigger_for_non_issue_file() {
  cd "$TMP_DIR"
  create_status "quick" "SPEC"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  # 非 issue 文件
  echo "# SPEC" > dev-doc/SPEC.md
  export TOOL_INPUT_FILE_PATH="dev-doc/SPEC.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "非 issue 文件不触发 audit" "quick" "$mode"
}

test_post_write_triggers_for_nested_issue_path() {
  # 测试 dev-doc/<branch>/issue/issue_*.md 路径格式
  cd "$TMP_DIR"
  create_status "fast" "TASK"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  mkdir -p dev-doc/issue
  echo "- [ ] Nested path bug" > dev-doc/issue/issue_test_2026-05-26_4.md

  export TOOL_INPUT_FILE_PATH="dev-doc/issue/issue_test_2026-05-26_4.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "TASK 阶段创建 issue 触发 audit/fast" "audit/fast" "$mode"
}

# =============================================
# TEST GROUP 5: iterate.sh audit 模式恢复逻辑
# =============================================
setup_iterate_env() {
  cd "$TMP_DIR"
  rm -rf dev-doc/task/* dev-doc/issue/* dev-doc/archive 2>/dev/null
  mkdir -p dev-doc/task dev-doc/issue
}

test_iterate_audit_quick_restores_quick() {
  setup_iterate_env
  create_status "audit/quick" "DEV"
  echo "2.0.0" > VERSION
  # audit 模式不需要 task 完成——跳过 task 检查
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-fix" "patch" "dev-doc" 2>&1)
  local rc=$?

  # 检查恢复后的 mode
  local mode phase
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')

  assert_eq "audit/quick iterate 后 mode 恢复 quick" "quick" "$mode"
  assert_eq "audit/quick iterate 后 phase 为 SPEC" "SPEC" "$phase"
}

test_iterate_audit_fast_restores_fast() {
  setup_iterate_env
  create_status "audit/fast" "DEV"
  echo "2.1.0" > VERSION
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-fix-2" "patch" "dev-doc" 2>&1

  local mode phase
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')

  assert_eq "audit/fast iterate 后 mode 恢复 fast" "fast" "$mode"
  assert_eq "audit/fast iterate 后 phase 为 TASK" "TASK" "$phase"
}

test_iterate_audit_full_restores_full() {
  setup_iterate_env
  create_status "audit/full" "DEV"
  echo "2.2.0" > VERSION
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-fix-3" "patch" "dev-doc" 2>&1

  local mode phase
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')

  assert_eq "audit/full iterate 后 mode 恢复 full" "full" "$mode"
  assert_eq "audit/full iterate 后 phase 为 PRD" "PRD" "$phase"
}

test_iterate_audit_mvp_restores_mvp() {
  setup_iterate_env
  create_status "audit/mvp" "DEV"
  echo "2.3.0" > VERSION
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-fix-4" "patch" "dev-doc" 2>&1

  local mode phase
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')

  assert_eq "audit/mvp iterate 后 mode 恢复 mvp" "mvp" "$mode"
  assert_eq "audit/mvp iterate 后 phase 为 SPEC" "SPEC" "$phase"
}

test_iterate_audit_invalid_original_defaults_quick() {
  setup_iterate_env
  create_status "audit/invalid_mode" "DEV"
  echo "2.4.0" > VERSION
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-fix-5" "patch" "dev-doc" 2>&1

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "audit/invalid iterate 后恢复为 quick（安全默认）" "quick" "$mode"
}

test_iterate_audit_skips_task_check() {
  # audit 模式下有未完成 task 仍然允许 iterate
  setup_iterate_env
  create_status "audit/quick" "DEV"
  echo "2.5.0" > VERSION

  # 创建一个未完成的 task
  cat > dev-doc/task/task_audit_1.md << 'TASK'
---
title: TASK - 一些任务
---

- [ ] T001: 未完成的任务
  - priority: P0
TASK

  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "audit-skip-task" "patch" "dev-doc" 2>&1)
  local rc=$?

  # 不应报错退出
  assert_not_contains "audit 模式跳过 task 检查不报 ERROR" "ERROR: 任务未全部完成" "$output"
}

test_iterate_non_audit_blocks_incomplete_task() {
  # 非 audit 模式下有未完成 task 阻断 iterate
  setup_iterate_env
  create_status "quick" "DEV"
  echo "2.6.0" > VERSION

  cat > dev-doc/task/task_normal_1.md << 'TASK'
---
title: TASK - 一些任务
---

- [ ] T001: 未完成的任务
  - priority: P0
- [x] T002: 已完成的任务
  - priority: P1
TASK

  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "normal-test" "patch" "dev-doc" 2>&1)
  local rc=$?

  assert_exit_code "非 audit 模式未完成 task 阻断 iterate" "1" "$rc"
  assert_contains "非 audit 报错信息包含任务未全部完成" "任务未全部完成" "$output"
}

# =============================================
# TEST GROUP 6: inject-context.sh audit 模式输出
# =============================================
test_inject_context_audit_mode_display() {
  cd "$TMP_DIR"
  create_status "audit/quick" "DEV"
  echo "2.0.0" > VERSION
  git tag -a "v2.0.0" -m "v2.0.0" 2>/dev/null || true
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/hooks/inject-context.sh 2>/dev/null)

  assert_contains "inject-context 输出包含 audit/quick" "audit/quick" "$output"
}

test_inject_context_audit_dev_hints() {
  cd "$TMP_DIR"
  create_status "audit/quick" "DEV"
  echo "2.0.0" > VERSION
  # 创建一个 open issue
  mkdir -p dev-doc/issue
  echo "- [ ] Some audit issue" > dev-doc/issue/issue_test_2026-05-26_1.md
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/hooks/inject-context.sh 2>/dev/null)

  assert_contains "audit DEV 输出包含恢复原模式提示" "恢复原模式" "$output"
}

test_inject_context_normal_mode_no_audit_hint() {
  cd "$TMP_DIR"
  create_status "quick" "DEV"
  echo "2.0.0" > VERSION
  mkdir -p dev-doc/task
  echo "- [x] T001: done" > dev-doc/task/done_task_1.md
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  local output
  output=$(bash scripts/hooks/inject-context.sh 2>/dev/null)

  assert_not_contains "非 audit 模式不显示恢复提示" "恢复原模式" "$output"
}

# =============================================
# TEST GROUP 7: 边界情况与错误处理
# =============================================
test_enter_audit_mode_output_message() {
  cd "$TMP_DIR"
  create_status "fast" "TASK"
  source scripts/lib/common.sh
  local output
  output=$(enter_audit_mode "dev-doc/STATUS.yaml")
  assert_contains "enter_audit_mode 输出包含原模式" "fast" "$output"
  assert_contains "enter_audit_mode 输出提及 audit" "audit" "$output"
}

test_post_write_phase_TEST_triggers_audit() {
  # TEST 阶段创建 issue 也应触发 audit
  cd "$TMP_DIR"
  create_status "quick" "TEST"
  git add -A && git commit -q -m "prep" 2>/dev/null || true

  mkdir -p dev-doc/issue
  echo "- [ ] Test phase bug" > dev-doc/issue/issue_test_2026-05-26_5.md

  export TOOL_INPUT_FILE_PATH="dev-doc/issue/issue_test_2026-05-26_5.md"
  bash scripts/hooks/post-write.sh 2>/dev/null
  unset TOOL_INPUT_FILE_PATH

  local mode
  mode=$(grep "^mode:" dev-doc/STATUS.yaml | sed 's/^mode: *//')
  assert_eq "TEST 阶段创建 issue 触发 audit/quick" "audit/quick" "$mode"
}

# =============================================
# 运行所有测试
# =============================================
setup_basic_env

echo "=== TEST GROUP 1: is_audit_mode 函数行为 ==="
test_is_audit_mode_positive
test_is_audit_mode_positive_full
test_is_audit_mode_negative_quick
test_is_audit_mode_negative_full
test_is_audit_mode_negative_fast
test_is_audit_mode_negative_empty
test_is_audit_mode_edge_audit_only

echo "=== TEST GROUP 2: enter_audit_mode 函数行为 ==="
test_enter_audit_mode_basic
test_enter_audit_mode_from_full
test_enter_audit_mode_already_audit_returns_1
test_enter_audit_mode_no_nested_audit

echo "=== TEST GROUP 3: mode.sh 拒绝 audit 输入 ==="
test_mode_rejects_audit
test_mode_rejects_audit_slash_quick
test_mode_accepts_quick
test_mode_accepts_fast
test_mode_rejects_audit_variant

echo "=== TEST GROUP 4: post-write.sh 触发条件 ==="
test_post_write_triggers_audit_on_issue_create
test_post_write_no_trigger_when_already_audit
test_post_write_no_trigger_when_phase_DEV
test_post_write_no_trigger_for_non_issue_file
test_post_write_triggers_for_nested_issue_path

echo "=== TEST GROUP 5: iterate.sh audit 模式恢复逻辑 ==="
test_iterate_audit_quick_restores_quick
test_iterate_audit_fast_restores_fast
test_iterate_audit_full_restores_full
test_iterate_audit_mvp_restores_mvp
test_iterate_audit_invalid_original_defaults_quick
test_iterate_audit_skips_task_check
test_iterate_non_audit_blocks_incomplete_task

echo "=== TEST GROUP 6: inject-context.sh audit 模式输出 ==="
test_inject_context_audit_mode_display
test_inject_context_audit_dev_hints
test_inject_context_normal_mode_no_audit_hint

echo "=== TEST GROUP 7: 边界情况与错误处理 ==="
test_enter_audit_mode_output_message
test_post_write_phase_TEST_triggers_audit

# =============================================
# 清理 & 汇总
# =============================================
rm -rf "$TMP_DIR"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━"
echo "测试结果: PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo -e "$ERRORS"
  echo ""
  echo "FAILED"
  exit 1
else
  echo "ALL PASSED"
  exit 0
fi
