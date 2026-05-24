#!/bin/bash
# 全量测试：验证 dev-flow v2.2 四项增强
# 覆盖：双重 Review / 连续执行 / 模型分级 / 交互支持
# 对应 task_2026-05-24_2.md (T1-T6)

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMMANDS_DIR="$SCRIPT_DIR/commands"
AGENTS_DIR="$SCRIPT_DIR/agents"
HOOKS_DIR="$SCRIPT_DIR/scripts/hooks"
TMP_DIR="$SCRIPT_DIR/tmp/test_v2_2_$$"
PASS=0; FAIL=0; ERRORS=""

# 辅助函数
assert_eq() {
  local actual="$1" expected="$2" msg="$3"
  if [ "$actual" = "$expected" ]; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected: [$expected]\n    got: [$actual]"
    echo "  FAIL: $msg (expected: [$expected], got: [$actual])"
  fi
}

assert_ge() {
  local actual="$1" limit="$2" msg="$3"
  if [ "$actual" -ge "$limit" ] 2>/dev/null; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected >= $limit, got: $actual"
    echo "  FAIL: $msg (expected >= $limit, got: $actual)"
  fi
}

assert_gt() {
  local actual="$1" limit="$2" msg="$3"
  if [ "$actual" -gt "$limit" ] 2>/dev/null; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected > $limit, got: $actual"
    echo "  FAIL: $msg (expected > $limit, got: $actual)"
  fi
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected to contain: [$expected]\n    output: [$(echo "$output" | head -5)]"
    echo "  FAIL: $msg (not found: $expected)"
  fi
}

assert_contains_regex() {
  local output="$1" pattern="$2" msg="$3"
  if echo "$output" | grep -qE "$pattern"; then
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected regex match: $pattern"
    echo "  FAIL: $msg (no regex match: $pattern)"
  fi
}

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if echo "$output" | grep -qF "$unexpected"; then
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain: $unexpected"
    echo "  FAIL: $msg (should NOT contain: $unexpected)"
  else
    PASS=$((PASS + 1))
    echo "  PASS: $msg"
  fi
}

# 准备临时目录
mkdir -p "$TMP_DIR"

echo "======================================================"
echo "= dev-flow v2.2 四项增强 - 全量测试"
echo "======================================================"
echo ""

# ============================================================
# T1: task-agent.md 增加 model hint 字段定义
# ============================================================
echo "--- T1: task-agent.md model hint 字段 ---"

TASK_AGENT_MD=$(cat "$AGENTS_DIR/task-agent.md")

# T1-1: model: 字段出现次数 >= 3
MODEL_COUNT=$(echo "$TASK_AGENT_MD" | grep -c "model:" || true)
assert_ge "$MODEL_COUNT" "3" "T1-1: task-agent.md 中 model: 出现次数 >= 3 (实际: $MODEL_COUNT)"

# T1-2: cheap/standard/capable 三个值都有定义
CHEAP_COUNT=$(echo "$TASK_AGENT_MD" | grep -c "cheap" || true)
STANDARD_COUNT=$(echo "$TASK_AGENT_MD" | grep -c "standard" || true)
CAPABLE_COUNT=$(echo "$TASK_AGENT_MD" | grep -c "capable" || true)
assert_gt "$CHEAP_COUNT" "0" "T1-2a: task-agent.md 包含 cheap"
assert_gt "$STANDARD_COUNT" "0" "T1-2b: task-agent.md 包含 standard"
assert_gt "$CAPABLE_COUNT" "0" "T1-2c: task-agent.md 包含 capable"

# T1-3: cheap/standard/capable 出现行数 >= 3
MODEL_VALUES=$(echo "$TASK_AGENT_MD" | grep -c "cheap\|standard\|capable" || true)
assert_ge "$MODEL_VALUES" "3" "T1-3: task-agent.md 中 cheap/standard/capable 出现行数 >= 3 (实际: $MODEL_VALUES)"

# T1-4: commands/task.md 中包含 model: 字段
TASK_CMD_MD=$(cat "$COMMANDS_DIR/task.md")
TASK_MODEL_COUNT=$(echo "$TASK_CMD_MD" | grep -c "model:" || true)
assert_ge "$TASK_MODEL_COUNT" "1" "T1-4: commands/task.md 中 model: 出现 >= 1 (实际: $TASK_MODEL_COUNT)"

# T1-5: task-agent.md 中有判断标准说明（<=2 文件、3-5 文件等）
assert_contains "$TASK_AGENT_MD" "<=2" "T1-5a: task-agent.md 包含 <=2 文件判断标准"
assert_contains "$TASK_AGENT_MD" "3-5" "T1-5b: task-agent.md 包含 3-5 文件判断标准"

# T1-6: model 字段是可选的，不填默认 standard
assert_contains "$TASK_AGENT_MD" "standard" "T1-6: task-agent.md 描述默认值 standard"

# T1-7: task 文件结构示例中包含 model 字段
# 验证 task 文件结构示例中含 model 字段（cheap | standard | capable 格式）
if echo "$TASK_AGENT_MD" | grep -qE "model:.*cheap" && echo "$TASK_AGENT_MD" | grep -qE "cheap.*standard.*capable"; then
  PASS=$((PASS + 1))
  echo "  PASS: T1-7: task 文件结构示例含 model 字段选项"
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: T1-7: task 文件结构示例含 model 字段选项"
  echo "  FAIL: T1-7: task 文件结构示例含 model 字段选项"
fi

echo ""

# ============================================================
# T2: STATUS.yaml exec_mode + inject-context 展示
# ============================================================
echo "--- T2: exec_mode + inject-context DEV[continuous] ---"

INJECT_SH="$HOOKS_DIR/inject-context.sh"

# T2-1: inject-context.sh 中读取 exec_mode
INJECT_CONTENT=$(cat "$INJECT_SH")
assert_contains "$INJECT_CONTENT" "exec_mode" "T2-1: inject-context.sh 读取 exec_mode"

# T2-2: 包含 DEV[continuous] 展示逻辑
assert_contains "$INJECT_CONTENT" "DEV[continuous]" "T2-2: inject-context.sh 包含 DEV[continuous] 展示"

# T2-3: 实际运行验证 - continuous 模式
mkdir -p "$TMP_DIR/proj_continuous/dev-doc/task"
cat > "$TMP_DIR/proj_continuous/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
exec_mode: continuous
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
# 创建一个任务文件以避免 BLOCKED 提示
cat > "$TMP_DIR/proj_continuous/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] T1：test task
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
# 需要 git repo 环境
cd "$TMP_DIR/proj_continuous" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_CONTINUOUS=$(cd "$TMP_DIR/proj_continuous" && bash "$INJECT_SH" 2>&1)
assert_contains "$OUTPUT_CONTINUOUS" "DEV[continuous]" "T2-3: continuous 模式输出包含 DEV[continuous]"

# T2-4: 实际运行验证 - step 模式（或无 exec_mode）
mkdir -p "$TMP_DIR/proj_step/dev-doc/task"
cat > "$TMP_DIR/proj_step/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_step/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] T1：test task
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_step" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_STEP=$(cd "$TMP_DIR/proj_step" && bash "$INJECT_SH" 2>&1)
assert_not_contains "$OUTPUT_STEP" "DEV[continuous]" "T2-4a: step 模式不输出 DEV[continuous]"
assert_contains "$OUTPUT_STEP" "DEV" "T2-4b: step 模式输出包含 DEV"
# 确保不含 [step] 后缀
assert_not_contains "$OUTPUT_STEP" "DEV[step]" "T2-4c: step 模式不输出 DEV[step]"

# T2-5: exec_mode: step 时也不展示后缀
mkdir -p "$TMP_DIR/proj_explicit_step/dev-doc/task"
cat > "$TMP_DIR/proj_explicit_step/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
exec_mode: step
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_explicit_step/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] T1：test task
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_explicit_step" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_EXPLICIT_STEP=$(cd "$TMP_DIR/proj_explicit_step" && bash "$INJECT_SH" 2>&1)
assert_not_contains "$OUTPUT_EXPLICIT_STEP" "[continuous]" "T2-5a: exec_mode=step 不展示 [continuous]"
assert_not_contains "$OUTPUT_EXPLICIT_STEP" "[step]" "T2-5b: exec_mode=step 不展示 [step]"

echo ""

# ============================================================
# T3: devtest.md 双重 Review
# ============================================================
echo "--- T3: devtest.md 双重 Review ---"

DEVTEST_MD=$(cat "$COMMANDS_DIR/devtest.md")

# T3-1: 包含 Round 1 / Round 2 关键字
ROUND_COUNT=$(echo "$DEVTEST_MD" | grep -c "Round 1\|Round 2\|Spec Compliance\|Code Quality" || true)
assert_ge "$ROUND_COUNT" "4" "T3-1: devtest.md 包含 Round 1/2/Spec Compliance/Code Quality >= 4 (实际: $ROUND_COUNT)"

# T3-2: 包含综合判定规则表
assert_contains "$DEVTEST_MD" "PASS" "T3-2a: devtest.md 包含 PASS 判定"
assert_contains "$DEVTEST_MD" "FAIL" "T3-2b: devtest.md 包含 FAIL 判定"
assert_contains "$DEVTEST_MD" "WARN" "T3-2c: devtest.md 包含 WARN 判定"

# T3-3: 包含两段独立 agent prompt 模板
# Round 1 模板
assert_contains "$DEVTEST_MD" "Spec Compliance" "T3-3a: devtest.md 包含 Spec Compliance agent"
# Round 2 模板
assert_contains "$DEVTEST_MD" "Code Quality" "T3-3b: devtest.md 包含 Code Quality agent"

# T3-4: Round 1 不评价代码质量
assert_contains "$DEVTEST_MD" "不评价代码质量" "T3-4: Round 1 明确不评价代码质量"

# T3-5: Round 2 包含四维评估
assert_contains "$DEVTEST_MD" "可读性" "T3-5a: Round 2 包含可读性维度"
assert_contains "$DEVTEST_MD" "可维护性" "T3-5b: Round 2 包含可维护性维度"
assert_contains "$DEVTEST_MD" "性能" "T3-5c: Round 2 包含性能维度"
assert_contains "$DEVTEST_MD" "安全" "T3-5d: Round 2 包含安全维度"

# T3-6: 综合判定表中 R1 PASS + R2 WARN → DONE_WITH_CONCERNS
assert_contains "$DEVTEST_MD" "DONE_WITH_CONCERNS" "T3-6a: devtest.md 包含 DONE_WITH_CONCERNS"
# 验证 R1 FAIL 时跳过 Round 2
assert_contains "$DEVTEST_MD" "BLOCKED" "T3-6b: devtest.md 包含 BLOCKED 状态"

# T3-7: Round 2 仅当 Round 1 PASS 时启动
assert_contains "$DEVTEST_MD" "仅当 Round 1" "T3-7: Round 2 明确依赖 Round 1 PASS"

# T3-8: 测试代码写入 tests/ 的要求保留
assert_contains "$DEVTEST_MD" "tests/" "T3-8: devtest.md 保留 tests/ 要求"

echo ""

# ============================================================
# T4: devtest.md Subagent 状态返回协议
# ============================================================
echo "--- T4: Subagent 状态返回协议 ---"

# T4-1: STATUS: DONE 格式出现
STATUS_DONE_COUNT=$(echo "$DEVTEST_MD" | grep -c "STATUS: DONE\|STATUS:.*DONE" || true)
assert_gt "$STATUS_DONE_COUNT" "0" "T4-1: devtest.md 包含 STATUS: DONE"

# T4-2: 四种状态总出现次数 >= 8
STATUS_ALL_COUNT=$(echo "$DEVTEST_MD" | grep -c "DONE\|DONE_WITH_CONCERNS\|NEEDS_CONTEXT\|BLOCKED" || true)
assert_ge "$STATUS_ALL_COUNT" "8" "T4-2: 四种状态出现次数 >= 8 (实际: $STATUS_ALL_COUNT)"

# T4-3: 向后兼容——无 STATUS 行视为 DONE
assert_contains "$DEVTEST_MD" "向后兼容" "T4-3a: devtest.md 描述向后兼容"
assert_contains_regex "$DEVTEST_MD" "无.*STATUS.*DONE|无 STATUS 行.*DONE" "T4-3b: 无 STATUS 行 = DONE"

# T4-4: Controller 行为定义
assert_contains "$DEVTEST_MD" "Controller" "T4-4a: devtest.md 定义 Controller 行为"
assert_contains "$DEVTEST_MD" "NEEDS_CONTEXT" "T4-4b: NEEDS_CONTEXT 状态定义"
assert_contains "$DEVTEST_MD" "BLOCKED" "T4-4c: BLOCKED 状态定义"

# T4-5: 各状态对应行为描述
assert_contains "$DEVTEST_MD" "暂停" "T4-5a: 包含暂停行为描述"
assert_contains "$DEVTEST_MD" "推进" "T4-5b: 包含推进行为描述"

# T4-6: DONE_WITH_CONCERNS 写入 issue
assert_contains_regex "$DEVTEST_MD" "DONE_WITH_CONCERNS.*issue|concerns.*issue" "T4-6: DONE_WITH_CONCERNS 关联 issue 写入"

# T4-7: STATUS 块格式包含 DETAIL
assert_contains "$DEVTEST_MD" "DETAIL" "T4-7: STATUS 块包含 DETAIL 字段"

echo ""

# ============================================================
# T5: devtest.md 连续执行模式
# ============================================================
echo "--- T5: devtest.md 连续执行模式 ---"

# T5-1: --continuous 和 --step 参数
CONTINUOUS_FLAG_COUNT=$(echo "$DEVTEST_MD" | grep -c "\-\-continuous\|\-\-step" || true)
assert_ge "$CONTINUOUS_FLAG_COUNT" "2" "T5-1: devtest.md 包含 --continuous / --step >= 2 (实际: $CONTINUOUS_FLAG_COUNT)"

# T5-2: exec_mode 引用
EXEC_MODE_COUNT=$(echo "$DEVTEST_MD" | grep -c "exec_mode" || true)
assert_ge "$EXEC_MODE_COUNT" "1" "T5-2: devtest.md 引用 exec_mode >= 1 (实际: $EXEC_MODE_COUNT)"

# T5-3: 连续推进规则——DONE 后自动下一个
assert_contains_regex "$DEVTEST_MD" "DONE.*自动|DONE.*下一个|DONE.*推进" "T5-3: DONE 后自动推进下一个 task"

# T5-4: 停顿条件描述
assert_contains "$DEVTEST_MD" "停顿" "T5-4: 包含停顿条件描述"

# T5-5: 所有 task 完成后提示 /test
assert_contains_regex "$DEVTEST_MD" "完成.*\/test|/test" "T5-5: 所有 task 完成后提示 /test"

# T5-6: post-write.sh 包含 continuous 相关提示
POST_WRITE_SH=$(cat "$HOOKS_DIR/post-write.sh")
POST_WRITE_CONTINUOUS_COUNT=$(echo "$POST_WRITE_SH" | grep -c "continuous" || true)
assert_ge "$POST_WRITE_CONTINUOUS_COUNT" "1" "T5-6: post-write.sh 包含 continuous (次数: $POST_WRITE_CONTINUOUS_COUNT)"

echo ""

# ============================================================
# T6: post-write.sh 连续模式下的触发提示调整
# ============================================================
echo "--- T6: post-write.sh 连续模式触发提示 ---"

# T6-1: post-write.sh 包含 continuous
assert_contains "$POST_WRITE_SH" "continuous" "T6-1: post-write.sh 包含 continuous"

# T6-2: post-write.sh 包含 exec_mode
EXEC_MODE_PW_COUNT=$(echo "$POST_WRITE_SH" | grep -c "exec_mode" || true)
assert_ge "$EXEC_MODE_PW_COUNT" "1" "T6-2: post-write.sh 包含 exec_mode (次数: $EXEC_MODE_PW_COUNT)"

# T6-3: continuous 模式下提示不同于 step 模式
# 验证有条件判断逻辑
assert_contains_regex "$POST_WRITE_SH" "continuous.*推进|continuous.*自动|continuous.*继续" "T6-3: continuous 提示含推进/自动/继续"

# T6-4: 实际运行验证 - continuous 模式 task 完成后的提示
mkdir -p "$TMP_DIR/proj_pw_cont/dev-doc/task"
cat > "$TMP_DIR/proj_pw_cont/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
exec_mode: continuous
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_pw_cont/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 2
---

- [x] T1：已完成任务
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test

- [ ] T2：未完成任务
  - level: P1
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_pw_cont" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_PW_CONT=$(cd "$TMP_DIR/proj_pw_cont" && TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$SCRIPT_DIR/scripts/hooks/post-write.sh" 2>&1)
assert_contains "$OUTPUT_PW_CONT" "continuous" "T6-4: continuous 模式下 post-write 提示含 continuous"

# T6-5: 实际运行验证 - step 模式 task 完成后的提示
mkdir -p "$TMP_DIR/proj_pw_step/dev-doc/task"
cat > "$TMP_DIR/proj_pw_step/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_pw_step/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 2
---

- [x] T1：已完成任务
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test

- [ ] T2：未完成任务
  - level: P1
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_pw_step" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_PW_STEP=$(cd "$TMP_DIR/proj_pw_step" && TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$SCRIPT_DIR/scripts/hooks/post-write.sh" 2>&1)
assert_not_contains "$OUTPUT_PW_STEP" "continuous" "T6-5: step 模式下 post-write 提示不含 continuous"
assert_contains "$OUTPUT_PW_STEP" "/devtest" "T6-5b: step 模式下提示 /devtest"

echo ""

# ============================================================
# SPEC 额外要求：向后兼容性验证
# ============================================================
echo "--- SPEC 向后兼容性验证 ---"

# 兼容-1: exec_mode 不存在时等同 step
# 已在 T2-4 中验证

# 兼容-2: model 字段可选，不填默认 standard
assert_contains "$TASK_AGENT_MD" "默认" "兼容-2a: task-agent.md 描述默认行为"
assert_contains_regex "$TASK_AGENT_MD" "不填.*standard|默认.*standard" "兼容-2b: 不填时默认 standard"

# 兼容-3: 无 STATUS 行 = DONE
# 已在 T4-3 中验证

# 兼容-4: SPEC 中定义双重 Review 始终启用（不可选）
# devtest.md 中不应有关闭双重 Review 的选项
assert_not_contains "$DEVTEST_MD" "关闭双重" "兼容-4a: 双重 Review 不可关闭"
assert_not_contains "$DEVTEST_MD" "禁用双重" "兼容-4b: 双重 Review 不可禁用"
# 使用 grep -F -- 避免 -- 被解释为选项
if echo "$DEVTEST_MD" | grep -qF -- "--no-review"; then
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 兼容-4c: 无 --no-review 选项\n    should NOT contain: --no-review"
  echo "  FAIL: 兼容-4c: 无 --no-review 选项 (should NOT contain: --no-review)"
else
  PASS=$((PASS + 1))
  echo "  PASS: 兼容-4c: 无 --no-review 选项"
fi

echo ""

# ============================================================
# SPEC 数据模型验证：STATUS.yaml exec_mode 字段规则
# ============================================================
echo "--- SPEC 数据模型：exec_mode 字段规则 ---"

# 数据-1: 默认值 step（不写等同 step）
# inject-context.sh 中对非 continuous 值的处理
assert_contains_regex "$INJECT_CONTENT" "continuous" "数据-1: inject-context.sh 检测 continuous 值"

# 数据-2: 仅 DEV 阶段有意义——非 DEV 阶段不展示 [continuous]
mkdir -p "$TMP_DIR/proj_test_phase/dev-doc"
cat > "$TMP_DIR/proj_test_phase/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: TEST
mode: full
exec_mode: continuous
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cd "$TMP_DIR/proj_test_phase" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_TEST_PHASE=$(cd "$TMP_DIR/proj_test_phase" && bash "$INJECT_SH" 2>&1)
# 在 TEST 阶段，即使 exec_mode=continuous，也不应特别展示（因为仅 DEV 有意义）
# 但代码实际逻辑是只在 PHASE=DEV 时才展示 DEV[continuous]
assert_not_contains "$OUTPUT_TEST_PHASE" "[continuous]" "数据-2: TEST 阶段不展示 [continuous]"

echo ""

# ============================================================
# SPEC 接口设计验证：devtest 命令接口
# ============================================================
echo "--- SPEC 接口设计：devtest 命令接口 ---"

# 接口-1: /devtest（无参数）为默认逐步模式
assert_contains "$DEVTEST_MD" "/devtest" "接口-1: devtest.md 包含 /devtest 命令"

# 接口-2: /devtest --continuous 切换（grep -F -- 避免 -- 被解释为选项）
if echo "$DEVTEST_MD" | grep -qF -- "--continuous"; then
  PASS=$((PASS + 1))
  echo "  PASS: 接口-2: devtest.md 包含 --continuous 参数"
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 接口-2: devtest.md 包含 --continuous 参数"
  echo "  FAIL: 接口-2: devtest.md 包含 --continuous 参数"
fi

# 接口-3: /devtest --step 切换回逐步模式（grep -F -- 避免 -- 被解释为选项）
if echo "$DEVTEST_MD" | grep -qF -- "--step"; then
  PASS=$((PASS + 1))
  echo "  PASS: 接口-3: devtest.md 包含 --step 参数"
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 接口-3: devtest.md 包含 --step 参数"
  echo "  FAIL: 接口-3: devtest.md 包含 --step 参数"
fi

# 接口-4: 切换操作更新 STATUS.yaml
assert_contains_regex "$DEVTEST_MD" "STATUS.yaml.*exec_mode|exec_mode.*STATUS" "接口-4: 切换操作更新 STATUS.yaml"

echo ""

# ============================================================
# SPEC 数据流验证：双重 Review 综合判定
# ============================================================
echo "--- SPEC 数据流：双重 Review 综合判定 ---"

# 流-1: R1 PASS + R2 PASS → DONE
assert_contains_regex "$DEVTEST_MD" "PASS.*PASS.*DONE|PASS.*\|.*PASS.*\|.*DONE" "流-1: R1 PASS + R2 PASS → DONE"

# 流-2: R1 FAIL → BLOCKED（跳过 R2）
assert_contains_regex "$DEVTEST_MD" "FAIL.*跳过|FAIL.*BLOCKED" "流-2: R1 FAIL → BLOCKED"

# 流-3: R1 PASS + R2 FAIL → BLOCKED / 写 issue
assert_contains_regex "$DEVTEST_MD" "PASS.*FAIL.*BLOCKED|R2.*FAIL" "流-3: R1 PASS + R2 FAIL 处理"

# 流-4: R2 仅 WARN → DONE_WITH_CONCERNS
assert_contains_regex "$DEVTEST_MD" "WARN.*DONE_WITH_CONCERNS|仅 WARN.*DONE_WITH_CONCERNS" "流-4: R2 WARN → DONE_WITH_CONCERNS"

echo ""

# ============================================================
# 边界场景：inject-context.sh 异常输入
# ============================================================
echo "--- 边界：inject-context.sh 异常输入 ---"

# 边界-1: exec_mode 值为非法值（不是 step/continuous）
mkdir -p "$TMP_DIR/proj_invalid_mode/dev-doc/task"
cat > "$TMP_DIR/proj_invalid_mode/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
exec_mode: invalid_value
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_invalid_mode/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] T1：test task
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_invalid_mode" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_INVALID=$(cd "$TMP_DIR/proj_invalid_mode" && bash "$INJECT_SH" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "边界-1a: 非法 exec_mode 值不崩溃"
assert_not_contains "$OUTPUT_INVALID" "[continuous]" "边界-1b: 非法值不展示 [continuous]"
assert_not_contains "$OUTPUT_INVALID" "[invalid_value]" "边界-1c: 非法值不展示原始值"

# 边界-2: exec_mode 为空值
mkdir -p "$TMP_DIR/proj_empty_mode/dev-doc/task"
cat > "$TMP_DIR/proj_empty_mode/dev-doc/STATUS.yaml" << 'EOF'
name: test-proj
phase: DEV
mode: full
exec_mode:
updated: 2026-05-24 12:00
started: 2026-05-24 12:00
EOF
cat > "$TMP_DIR/proj_empty_mode/dev-doc/task/task_2026-05-24_1.md" << 'EOF'
---
title: TASK - test
nums: 1
---

- [ ] T1：test task
  - level: P0
  - model: standard
  - details：test
  - depends on：无
  - Done when：test
EOF
cd "$TMP_DIR/proj_empty_mode" && git init -q && git add . && git commit -qm "init" 2>/dev/null
OUTPUT_EMPTY_MODE=$(cd "$TMP_DIR/proj_empty_mode" && bash "$INJECT_SH" 2>&1)
EXIT_CODE=$?
assert_eq "$EXIT_CODE" "0" "边界-2a: exec_mode 为空不崩溃"
assert_not_contains "$OUTPUT_EMPTY_MODE" "[continuous]" "边界-2b: 空 exec_mode 不展示 [continuous]"

echo ""

# ============================================================
# devtest.md 输入隔离规则验证
# ============================================================
echo "--- devtest.md 输入隔离规则 ---"

# 隔离-1: 禁止传入列表
assert_contains "$DEVTEST_MD" "禁止传入" "隔离-1: devtest.md 包含禁止传入列表"

# 隔离-2: 允许传入 context.sh 输出
assert_contains_regex "$DEVTEST_MD" "context.sh|项目上下文" "隔离-2: devtest.md 允许传入项目上下文"

# 隔离-3: 不允许传入 PRD.md
DEVTEST_FORBIDDEN=$(echo "$DEVTEST_MD" | grep "禁止传入" -A10)
assert_contains "$DEVTEST_FORBIDDEN" "PRD" "隔离-3: devtest.md 禁止传入 PRD.md"

echo ""

# ============================================================
# 连续执行模式 Controller 逻辑详细验证
# ============================================================
echo "--- 连续执行 Controller 逻辑 ---"

# Ctrl-1: 按 level 优先级推进（P0>P1>P2）
assert_contains_regex "$DEVTEST_MD" "P0.*P1.*P2|level.*优先|优先级" "Ctrl-1: 连续推进按 level 优先级"

# Ctrl-2: NEEDS_CONTEXT 停顿
assert_contains_regex "$DEVTEST_MD" "NEEDS_CONTEXT.*停顿|NEEDS_CONTEXT.*暂停|NEEDS_CONTEXT.*等待" "Ctrl-2: NEEDS_CONTEXT 导致停顿"

# Ctrl-3: BLOCKED 停顿
assert_contains_regex "$DEVTEST_MD" "BLOCKED.*停顿|BLOCKED.*暂停|BLOCKED.*写 issue" "Ctrl-3: BLOCKED 导致停顿"

# Ctrl-4: 结果处理中 BLOCKED 取消勾选
assert_contains_regex "$DEVTEST_MD" "BLOCKED.*取消|BLOCKED.*\[ \]|改回.*\[ \]" "Ctrl-4: BLOCKED 取消勾选"

echo ""

# ============================================================
# 清理
# ============================================================
rm -rf "$TMP_DIR"

# ============================================================
# 汇总
# ============================================================
echo "======================================================"
echo "= 测试结果汇总"
echo "======================================================"
echo "PASS: $PASS  FAIL: $FAIL  TOTAL: $((PASS + FAIL))"
if [ $FAIL -gt 0 ]; then
  echo ""
  echo "失败详情:"
  echo -e "$ERRORS"
  echo ""
  exit 1
fi
echo ""
echo "全部通过！"
exit 0
