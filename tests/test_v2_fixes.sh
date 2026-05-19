#!/bin/bash
# 测试 v2 迭代 7 个 P0 issue 修复的完整验证
# 覆盖：validate.sh 内容校验、阶段守卫、save-changelog 安全性、hooks.json 注册

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VALIDATE="$SCRIPT_DIR/scripts/init/validate.sh"
BLOCK_HOOK="$SCRIPT_DIR/scripts/hooks/block-non-dev-edit.sh"
SAVE_HOOK="$SCRIPT_DIR/scripts/hooks/save-changelog.sh"
INJECT_HOOK="$SCRIPT_DIR/scripts/hooks/inject-context.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_v2_fixes_$$"
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected NOT to contain: $unexpected"
  fi
}

assert_exit_code() {
  local actual="$1" expected="$2" msg="$3"
  if [ "$actual" -eq "$expected" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected exit code: $expected, got: $actual"
  fi
}

assert_file_not_contains() {
  local path="$1" unexpected="$2" msg="$3"
  if [ -f "$path" ] && ! grep -qF "$unexpected" "$path"; then
    PASS=$((PASS + 1))
  elif [ ! -f "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    file $path should NOT contain: $unexpected"
  fi
}

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content=""
    [ -f "$path" ] && content=$(head -20 "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    file: $path\n    expected to contain: $expected\n    content: $content"
  fi
}

echo "================================================================"
echo "=== 验证 Issue 1: validate.sh 内容结构校验 ==="
echo "================================================================"

# --- TEST 1.1: nums 不一致检测 ---
echo "TEST 1.1: nums 字段与实际条目数不一致"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 10
---
- [ ] I1：条目一
  - severity: P0
  - location：file.sh:10
  - description：描述
- [ ] I2：条目二
  - severity: P1
  - location：file.sh:20
  - description：描述2
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_nums_mismatch" "nums=10 实际=2 应检出不一致"

# --- TEST 1.2: nums 为 0 的边界 ---
echo "TEST 1.2: nums 为 0 但有条目"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 0
---
- [ ] I1：条目
  - severity: P0
  - location：file.sh:10
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_nums_mismatch" "nums=0 实际=1 应检出不一致"

# --- TEST 1.3: 条目格式缺少全角冒号 ---
echo "TEST 1.3: 条目使用半角冒号而非全角"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 1
---
- [ ] I1:半角冒号标题
  - severity: P0
  - location：file.sh:10
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_bad_item_format" "半角冒号应被检测为格式错误"

# --- TEST 1.4: 条目格式 I 后非数字 ---
echo "TEST 1.4: I后非数字（如 IA）"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 1
---
- [ ] IA：非数字编号
  - severity: P0
  - location：file.sh:10
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_bad_item_format" "I后非数字应被检测为格式错误"

# --- TEST 1.5: severity 非法值检测（各种非法值）---
echo "TEST 1.5: severity 非法值(HIGH/CRITICAL/p0)"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 3
---
- [ ] I1：非法一
  - severity: HIGH
  - location：file.sh:10
  - description：描述
- [ ] I2：非法二
  - severity: CRITICAL
  - location：file.sh:20
  - description：描述
- [ ] I3：非法三
  - severity: p0
  - location：file.sh:30
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_invalid_severity" "非法 severity 应被检测"

# --- TEST 1.6: 混合合法与非法 severity ---
echo "TEST 1.6: 混合合法与非法 severity"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 2
---
- [ ] I1：合法
  - severity: P0
  - location：file.sh:10
  - description：描述
- [ ] I2：非法
  - severity: LOW
  - location：file.sh:20
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_invalid_severity" "有一个非法 severity 就应报告"

# --- TEST 1.7: 缺少所有必需子字段 ---
echo "TEST 1.7: 条目完全无子字段"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 1
---
- [ ] I1：无子字段条目
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_missing_required_fields" "完全无子字段应被检测"

# --- TEST 1.8: 部分缺失（只有 severity，缺 location 和 description）---
echo "TEST 1.8: 只有 severity 缺其他两个"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 1
---
- [ ] I1：部分字段
  - severity: P1
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_missing_required_fields" "缺少 location 和 description 应报告"

# --- TEST 1.9: 多条目，部分有问题部分没问题 ---
echo "TEST 1.9: 多条目混合（一个完整一个缺字段）"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 2
---
- [ ] I1：完整条目
  - severity: P0
  - location：file.sh:10
  - description：描述完整
- [ ] I2：缺字段条目
  - severity: P1
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_missing_required_fields" "部分条目缺字段应报告"

echo ""
echo "================================================================"
echo "=== 验证 Issue 4/7: 阶段守卫 block-non-dev-edit.sh ==="
echo "================================================================"

# --- TEST 2.1: PRD 阶段阻断源码修改 ---
echo "TEST 2.1: PRD 阶段阻断"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: PRD
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "src/main.py"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 2 "PRD 阶段写源码应阻断"

# --- TEST 2.2: SPEC 阶段阻断 ---
echo "TEST 2.2: SPEC 阶段阻断"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: SPEC
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "lib/util.js"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 2 "SPEC 阶段写源码应阻断"

# --- TEST 2.3: DONE 阶段阻断（核心验证点） ---
echo "TEST 2.3: DONE 阶段阻断"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "app/controller.rb"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 2 "DONE 阶段写源码应阻断"

# --- TEST 2.4: DEV 阶段放行 ---
echo "TEST 2.4: DEV 阶段放行"
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
CLAUDE_TOOL_INPUT='{"file_path": "src/main.py"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 0 "DEV 阶段写源码应放行"

# --- TEST 2.5: 白名单 dev-doc 放行（所有阶段） ---
echo "TEST 2.5: DONE 阶段写 dev-doc/ 放行"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "dev-doc/TEST.md"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 0 "DONE 阶段写 dev-doc/ 应放行"

# --- TEST 2.6: 白名单 tests/ 放行 ---
echo "TEST 2.6: TEST 阶段写 tests/ 放行"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: TEST
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "tests/test_auth.py"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 0 "TEST 阶段写 tests/ 应放行"

# --- TEST 2.7: 无 STATUS.yaml 时放行 ---
echo "TEST 2.7: 无 STATUS.yaml 时放行"
setup
CLAUDE_TOOL_INPUT='{"file_path": "src/main.py"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 0 "无 STATUS.yaml 时应放行"

# --- TEST 2.8: 空 TOOL_INPUT 时放行 ---
echo "TEST 2.8: 无法提取 file_path 时放行"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 0 "无 file_path 时应放行"

echo ""
echo "================================================================"
echo "=== 验证 Issue 6: save-changelog.sh 安全性 ==="
echo "================================================================"

# --- TEST 3.1: 不使用 sed -i ---
echo "TEST 3.1: save-changelog.sh 不使用 sed -i"
if grep -q "sed -i" "$SAVE_HOOK"; then
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: save-changelog.sh 不应使用 sed -i"
else
  PASS=$((PASS + 1))
fi

# --- TEST 3.2: CHANGELOG 不变 binary ---
echo "TEST 3.2: CHANGELOG 输出为文本文件（非 binary）"
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
bash "$SAVE_HOOK" > /dev/null 2>&1
FILE_TYPE=$(file dev-doc/CHANGELOG.md)
if echo "$FILE_TYPE" | grep -qi "text"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: CHANGELOG 应为文本文件\n    file type: $FILE_TYPE"
fi

# --- TEST 3.3: 多次运行不产生 binary ---
echo "TEST 3.3: 多次运行后仍为文本"
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
for i in $(seq 1 5); do
  bash "$SAVE_HOOK" > /dev/null 2>&1
done
FILE_TYPE=$(file dev-doc/CHANGELOG.md)
if echo "$FILE_TYPE" | grep -qi "text"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 多次运行后 CHANGELOG 应仍为文本\n    file type: $FILE_TYPE"
fi

# --- TEST 3.4: 中文 commit message 处理 ---
echo "TEST 3.4: 中文 commit message 不应导致 topic 为空"
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
git commit --allow-empty -m "修复登录验证逻辑" -q
bash "$SAVE_HOOK" > /dev/null 2>&1
# topic 不应为空（检查 CHANGELOG 最后一行 - 后是否有非空内容）
LAST_LINE=$(grep "^- " dev-doc/CHANGELOG.md | tail -1)
# 格式应为 "- HH:MM <topic>"，topic 不应为空
TOPIC_PART=$(echo "$LAST_LINE" | sed 's/^- [0-9]\{2\}:[0-9]\{2\} //')
if [ -n "$TOPIC_PART" ] && [ "$TOPIC_PART" != "$LAST_LINE" ]; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 中文 commit message 应正确记录为 topic\n    last line: $LAST_LINE\n    topic part: $TOPIC_PART"
fi

# --- TEST 3.5: inject-context.sh 的 grep 有 -a flag ---
echo "TEST 3.5: inject-context.sh grep CHANGELOG 有 -a flag"
if grep -q 'grep -a' "$INJECT_HOOK"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: inject-context.sh 应使用 grep -a 读取 CHANGELOG"
fi

echo ""
echo "================================================================"
echo "=== 验证 Issue 7: hooks.json 注册 ==="
echo "================================================================"

# --- TEST 4.1: block-non-dev-edit.sh 在 PreToolUse Write|Edit 中注册 ---
echo "TEST 4.1: hooks.json 注册 block-non-dev-edit.sh"
HOOKS_JSON="$SCRIPT_DIR/hooks.json"
if grep -q "block-non-dev-edit" "$HOOKS_JSON"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: hooks.json 应注册 block-non-dev-edit.sh"
fi

# --- TEST 4.2: PreToolUse Write|Edit matcher 正确 ---
echo "TEST 4.2: PreToolUse matcher 包含 Write|Edit"
# 检查 block-non-dev-edit 是否在 Write|Edit matcher 下
if python3 -c "
import json, sys
with open('$HOOKS_JSON') as f:
    data = json.load(f)
pre = data.get('hooks', {}).get('PreToolUse', [])
found = False
for entry in pre:
    if 'Write' in entry.get('matcher', '') and 'Edit' in entry.get('matcher', ''):
        for hook in entry.get('hooks', []):
            if 'block-non-dev-edit' in hook.get('command', ''):
                found = True
                break
if found:
    sys.exit(0)
else:
    sys.exit(1)
" 2>/dev/null; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: block-non-dev-edit.sh 应在 PreToolUse Write|Edit matcher 下"
fi

echo ""
echo "================================================================"
echo "=== 验证 Issue 2: init.md 规范对照指令 ==="
echo "================================================================"

# --- TEST 5.1: init.md 阶段 3 包含规范对照要求 ---
echo "TEST 5.1: init.md 有规范对照要求"
INIT_MD="$SCRIPT_DIR/commands/init.md"
if grep -q "规范对照" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: commands/init.md 应包含规范对照要求"
fi

# --- TEST 5.2: 规范对照指向 references/dev-doc/ ---
echo "TEST 5.2: 规范对照指向正确路径"
if grep -q "references/dev-doc/" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 规范对照应指向 references/dev-doc/ 目录"
fi

echo ""
echo "================================================================"
echo "=== 验证 Issue 5: init.md warning 处理指令 ==="
echo "================================================================"

# --- TEST 6.1: issue_nums_mismatch 处理 ---
echo "TEST 6.1: init.md 有 issue_nums_mismatch 处理"
if grep -q "issue_nums_mismatch" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: init.md 应有 issue_nums_mismatch 处理"
fi

# --- TEST 6.2: issue_bad_item_format 处理 ---
echo "TEST 6.2: init.md 有 issue_bad_item_format 处理"
if grep -q "issue_bad_item_format" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: init.md 应有 issue_bad_item_format 处理"
fi

# --- TEST 6.3: issue_missing_required_fields 处理 ---
echo "TEST 6.3: init.md 有 issue_missing_required_fields 处理"
if grep -q "issue_missing_required_fields" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: init.md 应有 issue_missing_required_fields 处理"
fi

# --- TEST 6.4: issue_invalid_severity 处理 ---
echo "TEST 6.4: init.md 有 issue_invalid_severity 处理"
if grep -q "issue_invalid_severity" "$INIT_MD"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: init.md 应有 issue_invalid_severity 处理"
fi

echo ""
echo "================================================================"
echo "=== 验证 Issue 3: 测试覆盖（test_validate.sh 有 TEST 9-12）==="
echo "================================================================"

# --- TEST 7.1: test_validate.sh 有 TEST 9 (条目格式) ---
echo "TEST 7.1: test_validate.sh 有 TEST 9"
TEST_VALIDATE="$SCRIPT_DIR/tests/test_validate.sh"
if grep -q "TEST 9" "$TEST_VALIDATE"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: test_validate.sh 应有 TEST 9"
fi

# --- TEST 7.2: test_validate.sh 有 TEST 10 (必需子字段) ---
echo "TEST 7.2: test_validate.sh 有 TEST 10"
if grep -q "TEST 10" "$TEST_VALIDATE"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: test_validate.sh 应有 TEST 10"
fi

# --- TEST 7.3: test_validate.sh 有 TEST 11 (nums 一致性) ---
echo "TEST 7.3: test_validate.sh 有 TEST 11"
if grep -q "TEST 11" "$TEST_VALIDATE"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: test_validate.sh 应有 TEST 11"
fi

# --- TEST 7.4: test_validate.sh 有 TEST 12 (severity 合法值) ---
echo "TEST 7.4: test_validate.sh 有 TEST 12"
if grep -q "TEST 12" "$TEST_VALIDATE"; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: test_validate.sh 应有 TEST 12"
fi

echo ""
echo "================================================================"
echo "=== 边界情况测试 ==="
echo "================================================================"

# --- TEST 8.1: 空 issue 文件（无条目）不触发内容校验 ---
echo "TEST 8.1: 空 issue 文件不触发内容校验"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 0
---
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_not_contains "$OUTPUT" "issue_bad_item_format" "空文件不应报 bad_item_format"
assert_not_contains "$OUTPUT" "issue_missing_required_fields" "空文件不应报 missing_required_fields"
assert_not_contains "$OUTPUT" "issue_invalid_severity" "空文件不应报 invalid_severity"

# --- TEST 8.2: [x] 已关闭条目也参与格式校验 ---
echo "TEST 8.2: 已关闭条目也应校验格式"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 2
---
- [ ] I1：未关闭条目
  - severity: P0
  - location：file.sh:10
  - description：描述
- [x] 无编号已关闭条目
  - severity: P1
  - location：file.sh:20
  - description：描述
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_bad_item_format" "已关闭条目的格式错误也应检测"

# --- TEST 8.3: save-changelog.sh 日期段不重复（幂等性） ---
echo "TEST 8.3: save-changelog.sh 幂等性"
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
bash "$SAVE_HOOK" > /dev/null 2>&1
bash "$SAVE_HOOK" > /dev/null 2>&1
bash "$SAVE_HOOK" > /dev/null 2>&1
TODAY=$(date +%Y-%m-%d)
DATE_COUNT=$(grep -c "^## $TODAY" dev-doc/CHANGELOG.md)
if [ "$DATE_COUNT" -eq 1 ]; then
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + 1))
  ERRORS="$ERRORS\n  FAIL: 多次运行应只产生一个日期段，实际有 $DATE_COUNT 个"
fi

# --- TEST 8.4: block-non-dev-edit.sh TASK 阶段阻断 ---
echo "TEST 8.4: TASK 阶段阻断"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: TASK
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
CLAUDE_TOOL_INPUT='{"file_path": "index.html"}' bash "$BLOCK_HOOK" 2>&1
assert_exit_code $? 2 "TASK 阶段写源码应阻断"

# --- TEST 8.5: validate.sh 检测 [x] 条目的 severity 合法性 ---
echo "TEST 8.5: [x] 条目的 severity 也应校验"
setup
mkdir -p dev-doc/issue
cat > "dev-doc/issue/issue_test_2026-05-15_1.md" << 'EOF'
---
source: test
nums: 1
---
- [x] I1：已关闭但 severity 非法
  - severity: MAJOR
  - location：file.sh:10
  - description：描述
  - fix：已修复
EOF
OUTPUT=$(bash "$VALIDATE" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_invalid_severity" "已关闭条目的非法 severity 也应检测"

# === 汇总 ===
teardown
echo ""
echo "================================================================"
echo "=== v2 修复验证 测试结果 ==="
echo "================================================================"
echo "PASS: $PASS  FAIL: $FAIL  TOTAL: $((PASS + FAIL))"
if [ $FAIL -gt 0 ]; then
  echo -e "\n失败详情：$ERRORS"
  exit 1
fi
exit 0
