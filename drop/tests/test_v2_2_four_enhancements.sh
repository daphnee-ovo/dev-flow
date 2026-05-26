#!/bin/bash
# 验证轻量化后的四项增强：
# task complexity、exec_mode、devtest 三状态、连续模式提示。

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMMANDS_DIR="$SCRIPT_DIR/commands"
AGENTS_DIR="$SCRIPT_DIR/agents"
HOOKS_DIR="$SCRIPT_DIR/scripts/hooks"
DEVTEST_SCRIPT="$SCRIPT_DIR/scripts/commands/devtest.sh"
WORK_DIR="$SCRIPT_DIR/temp/test_v2_2_$$"
PASS=0
FAIL=0
ERRORS=""

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF -- "$expected"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: $expected"
    echo "  FAIL: $msg"
  fi
}

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if echo "$output" | grep -qF -- "$unexpected"; then
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    unexpected: $unexpected"
    echo "  FAIL: $msg"
  else
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  fi
}

assert_file_contains() {
  local file="$1" expected="$2" msg="$3"
  if [ -f "$file" ] && grep -qF -- "$expected" "$file"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected in $file: $expected"
    echo "  FAIL: $msg"
  fi
}

cleanup() {
  cd "$SCRIPT_DIR"
  if command -v trash >/dev/null 2>&1; then
    trash "$WORK_DIR" >/dev/null 2>&1 || true
  elif [ -d "$WORK_DIR" ]; then
    mv "$WORK_DIR" "$SCRIPT_DIR/temp/test_v2_2_done_$$" 2>/dev/null || true
  fi
}

echo "======================================================"
echo "= dev-flow v2.2 轻量增强测试"
echo "======================================================"

TASK_AGENT_MD=$(cat "$AGENTS_DIR/task-agent.md")
TASK_CMD_MD=$(cat "$COMMANDS_DIR/task.md")
DEVTEST_MD=$(cat "$COMMANDS_DIR/devtest.md")

echo "--- T1: task 模型轻量化 ---"
# task-agent.md 引用 references/，格式定义在 references/dev-doc/TASK-FILE.md 中
assert_contains "$TASK_AGENT_MD" "references/dev-doc/TASK-FILE.md" "task-agent 引用 references"
TASK_FILE_REF=$(cat "$SCRIPT_DIR/references/dev-doc/TASK-FILE.md")
assert_contains "$TASK_FILE_REF" "complexity:" "TASK-FILE 包含 complexity"
assert_contains "$TASK_FILE_REF" "refs:" "TASK-FILE 包含 refs"
assert_contains "$TASK_FILE_REF" "files:" "TASK-FILE 包含 files"
assert_contains "$TASK_FILE_REF" "done_when" "TASK-FILE 包含 done_when"
assert_not_contains "$TASK_CMD_MD" "model: cheap" "commands/task.md 不再要求 model"
assert_contains "$TASK_CMD_MD" "不要增加 model/steps/verification/docs 字段" "commands/task.md 明确剪枝字段"

echo "--- T2: exec_mode + inject-context ---"
INJECT_CONTENT=$(cat "$HOOKS_DIR/inject-context.sh")
assert_contains "$INJECT_CONTENT" "exec_mode" "inject-context 读取 exec_mode"
assert_contains "$INJECT_CONTENT" "DEV[continuous]" "inject-context 支持 continuous 展示"

mkdir -p "$WORK_DIR/proj/dev-doc/task"
cat > "$WORK_DIR/proj/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: quick
exec_mode: continuous
updated: 2026-05-26 00:00
started: 2026-05-26 00:00
EOF
cat > "$WORK_DIR/proj/dev-doc/task/task_2026-05-26_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] TASK-T001: test
  - priority: P0
  - refs: user-request
  - files:
      test: ["tests/test_sample.sh"]
  - depends_on: []
  - complexity: S
  - done_when:
      - pass
EOF
cd "$WORK_DIR/proj" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_CONTINUOUS=$(cd "$WORK_DIR/proj" && bash "$HOOKS_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT_CONTINUOUS" "DEV[continuous]" "continuous 模式输出 DEV[continuous]"

echo "--- T3: devtest 最小三状态 ---"
assert_contains "$DEVTEST_MD" "PASS" "devtest.md 包含 PASS"
assert_contains "$DEVTEST_MD" "FAIL" "devtest.md 包含 FAIL"
assert_contains "$DEVTEST_MD" "NEEDS_CONTEXT" "devtest.md 包含 NEEDS_CONTEXT"
assert_not_contains "$DEVTEST_MD" "DONE_WITH_CONCERNS" "devtest.md 不再保留大型 controller 状态"
assert_contains "$DEVTEST_MD" "scripts/commands/devtest.sh" "devtest.md 指向可执行脚本"

mkdir -p "$WORK_DIR/devtest/dev-doc/task"
cat > "$WORK_DIR/devtest/dev-doc/STATUS.yaml" << 'EOF'
name: devtest
phase: DEV
mode: quick
updated: 2026-05-26 00:00
started: 2026-05-26 00:00
EOF
cat > "$WORK_DIR/devtest/dev-doc/task/task_2026-05-26_1.md" << 'EOF'
---
title: TASK - devtest
nums: 1
---

- [x] TASK-T001: done
  - priority: P0
  - refs: user-request
  - files:
      test: ["tests/test_sample.sh"]
  - depends_on: []
  - complexity: S
  - done_when:
      - pass
EOF
OUTPUT_PASS=$(cd "$WORK_DIR/devtest" && bash "$DEVTEST_SCRIPT" --result PASS dev-doc 2>&1)
assert_contains "$OUTPUT_PASS" "devtest PASS" "devtest 脚本 PASS 可执行"
OUTPUT_MODE=$(cd "$WORK_DIR/devtest" && bash "$DEVTEST_SCRIPT" --continuous dev-doc 2>&1)
assert_file_contains "$WORK_DIR/devtest/dev-doc/STATUS.yaml" "exec_mode: continuous" "devtest 脚本可切换 continuous"

echo "--- T4: post-write 连续提示 ---"
POST_WRITE=$(cat "$HOOKS_DIR/post-write.sh")
assert_contains "$POST_WRITE" "continuous" "post-write 包含 continuous 提示"
assert_contains "$POST_WRITE" "自动推进" "continuous 提示自动推进"

cleanup

echo ""
echo "======================================================"
echo "= 测试结果汇总"
echo "======================================================"
echo "PASS: $PASS  FAIL: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
echo "全部通过！"
exit 0
