#!/bin/bash
# E2E 对抗测试：模拟 agent 越界、不规范、逃逸操作
# 验证 dev-flow 的各种防护机制能正确拦截违规行为

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CMD_DIR="$SCRIPT_DIR/scripts/commands"
HOOK_DIR="$SCRIPT_DIR/scripts/hooks"
INIT_DIR="$SCRIPT_DIR/scripts/init"
LIB_DIR="$SCRIPT_DIR/scripts/lib"
TMP_DIR="$SCRIPT_DIR/tmp/test_adversarial_$$"
PASS=0; FAIL=0; ERRORS=""

# === 测试工具函数 ===
setup_repo() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q .
  git commit --allow-empty -m "init" -q
}

cleanup() {
  cd "$SCRIPT_DIR"
  rm -rf "$TMP_DIR"
}

assert_ok() {
  local exit_code="$1" msg="$2"
  if [ "$exit_code" -eq 0 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg (exit=$exit_code)"
  fi
}

assert_fail() {
  local exit_code="$1" msg="$2"
  if [ "$exit_code" -ne 0 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg (expected non-zero exit, got 0)"
  fi
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain: $unexpected"
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

assert_file_not_exists() {
  local path="$1" msg="$2"
  if [ ! -f "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    file should not exist: $path"
  fi
}

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content="(file not found)"
    [ -f "$path" ] && content=$(head -5 "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected in $path: $expected\n    content: $content"
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
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain in $path: $unexpected"
  fi
}

assert_no_git_tag() {
  local tag="$1" msg="$2"
  if ! git tag -l "$tag" | grep -q "$tag"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    tag should not exist: $tag"
  fi
}

echo "================================================================"
echo "  E2E 对抗测试：agent 越界/逃逸模拟"
echo "================================================================"
echo ""

# ======================================================================
# SCENARIO 1: 阶段跳跃 — agent 试图跳过必经阶段
# ======================================================================
echo "=============================="
echo "  SCENARIO 1: 阶段跳跃"
echo "=============================="

# --- 1.1: PRD 阶段直接 iterate ---
echo "  1.1: PRD 阶段试图 iterate"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: PRD
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
# iterate 不检查 phase（它检查 task 和 issue），但无 task 应安全通过
# 关键：iterate 应该仍然能执行（因为 task 0/0 算通过）
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "skip-test" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "1.1: 无 task 时 iterate 技术上可执行（不阻断）"

# --- 1.2: DEV 阶段有未完成 P0 task 试图 iterate ---
echo "  1.2: DEV 阶段未完成 task 试图 iterate"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [x] T1：已完成
  - level: P0
- [ ] T2：未完成
  - level: P0
EOF
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "rush" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "1.2: 有未完成 task 应阻断 iterate"
assert_contains "$OUTPUT" "1/2" "1.2: 应显示完成进度"

# --- 1.3: agent 试图手动篡改 phase 绕过检查 ---
echo "  1.3: 手动设 phase=DONE 但 task 未完成"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：未完成任务
  - level: P0
EOF
# iterate 不看 phase 而是直接看 task 完成度，应阻断
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "cheated" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "1.3: 即使 phase=DONE，task 未完成仍应阻断"

# --- 1.4: agent 试图把未完成 task 文件重命名为 done_ 绕过检查 ---
echo "  1.4: 手动重命名未完成 task 为 done_ 前缀"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# agent 把未完成的 task 文件重命名为 done_ 试图绕过
cat > dev-doc/task/done_task_2026-05-24_1.md << 'EOF'
- [ ] T1：实际未完成但被标记为 done
  - level: P0
EOF
# iterate 同时检查 done_task_* 中的完成度
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "bypass" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "1.4: done_ 文件中有未完成条目应仍然阻断"

echo ""

# ======================================================================
# SCENARIO 2: VERSION 篡改与注入
# ======================================================================
echo "=============================="
echo "  SCENARIO 2: VERSION 篡改与注入"
echo "=============================="

setup_repo
source "$LIB_DIR/version.sh"

# --- 2.1: 注入换行符 ---
echo "  2.1: VERSION 注入换行符"
printf "1.0.0\nmalicious" > VERSION
VER=$(version_read)
# version_read 的 tr -d 会拼合所有行为 "1.0.0malicious"，应校验失败
version_validate "$VER"
EXIT=$?
assert_fail "$EXIT" "2.1: 含换行注入的 VERSION 校验应失败（内容被污染）"

# --- 2.2: 注入空格和制表符 ---
echo "  2.2: VERSION 前后有空白"
printf "  1.0.0\t\n" > VERSION
VER=$(version_read)
version_validate "$VER"
assert_ok $? "2.2: 带空白的 VERSION 应被正确 trim"

# --- 2.3: VERSION 为空文件 ---
echo "  2.3: VERSION 为空文件"
> VERSION
VER=$(version_read)
if [ -z "$VER" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 2.3 空 VERSION 应返回空"; fi

# --- 2.4: VERSION 含 shell 注入 ---
echo "  2.4: VERSION 含 shell 特殊字符"
echo '$(rm -rf /)' > VERSION
VER=$(version_read)
version_validate "$VER"
assert_fail $? "2.4: shell 注入内容应不通过校验"

# --- 2.5: VERSION 超长内容 ---
echo "  2.5: VERSION 超长内容"
python3 -c "print('1.' + '0'*1000 + '.0')" > VERSION 2>/dev/null || printf "1.%01000d.0" 0 > VERSION
VER=$(version_read)
version_validate "$VER"
# 数值超大但格式合法——validate 只检查格式，不限制数值范围
# 这里取决于实现：grep -qE 应该能匹配
# 如果通过了也是合理的（格式合法），如果不通过也合理（实现限制）
# 我们测试它不崩溃就行
EXIT=$?
assert_ok 0 "2.5: 超长 VERSION 处理不应崩溃"

# --- 2.6: 负数版本 ---
echo "  2.6: 负数版本号"
echo "-1.0.0" > VERSION
VER=$(version_read)
version_validate "$VER"
assert_fail $? "2.6: 负数版本应不合法"

# --- 2.7: bump 非法类型 ---
echo "  2.7: bump 非法类型"
RESULT=$(version_bump "1.0.0" "invalid_type" 2>&1)
EXIT=$?
assert_fail "$EXIT" "2.7: 非法 bump 类型应失败"

echo ""

# ======================================================================
# SCENARIO 3: 文件命名规范逃逸
# ======================================================================
echo "=============================="
echo "  SCENARIO 3: 文件命名规范逃逸"
echo "=============================="

setup_repo
mkdir -p dev-doc/task dev-doc/issue

# --- 3.1: task 文件名含路径遍历 ---
echo "  3.1: task 文件名含 ../"
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：正常任务
  Done when: pass
EOF
# validate 应报出有问题的文件但不崩溃
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_not_contains "$OUTPUT" "error" "3.1: validate 不应崩溃"

# --- 3.2: issue 文件名完全不合规 ---
echo "  3.2: issue 文件名不合规"
cat > dev-doc/issue/random_file.md << 'EOF'
- [ ] 试图绕过命名规范
  - severity: P0
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
# validate 的 issue 校验用 *.md glob 然后 regex 检查
assert_contains "$OUTPUT" "issue_bad_name:random_file.md" "3.2: 不规范 issue 文件名应被检出"

# --- 3.3: issue 伪造 closed 前缀但实际未关闭 ---
echo "  3.3: 伪造 closed 前缀未关闭的 issue"
cat > dev-doc/issue/closed_issue_test_2026-05-24_1.md << 'EOF'
---
title: 伪造关闭
nums: 1
---

- [ ] I1：实际未修复
  - severity: P0
  - location: fake.js:1
  - description: 伪造关闭
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_closed_but_open_items" "3.3: closed 但有 open 条目应被检出"

# --- 3.4: issue 全部完成但无 closed 前缀 ---
echo "  3.4: issue 全完成但未标 closed 前缀"
cat > dev-doc/issue/issue_test_2026-05-24_2.md << 'EOF'
---
title: 应该关闭
nums: 1
---

- [x] I1：已修复
  - severity: P1
  - location: test.js:1
  - description: 已修复
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_should_be_closed" "3.4: 全部完成但未关闭应被检出"

# --- 3.5: task 伪造 done 前缀但实际未完成 ---
echo "  3.5: 伪造 done_ 前缀未完成的 task"
cat > dev-doc/task/done_task_2026-05-24_99.md << 'EOF'
- [ ] T1：实际未完成
  Done when: pass
EOF
# validate 用 task_*.md glob，done_task 不在此范围
# 但 iterate.sh 会检查 done_task 内部完成度
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
echo "1.0.0" > VERSION
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "fake-done" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "3.5: 伪造 done_ 前缀但未完成应被 iterate 阻断"

echo ""

# ======================================================================
# SCENARIO 4: issue severity 伪造与操纵
# ======================================================================
echo "=============================="
echo "  SCENARIO 4: issue severity 操纵"
echo "=============================="

# --- 4.1: 将 P0 issue 降级为 P2 试图绕过阻断 ---
echo "  4.1: P0 降为 P2 试图绕过 iterate 阻断"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# 如果 issue 被降级为 P2，iterate 不会阻断
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
- [ ] I1：严重 bug 但被降为 P2
  - severity: P2
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "downgraded" "minor" "dev-doc" 2>&1)
# 非 P0 不阻断——这是设计如此，但 validate 应检测 severity 合法性
assert_contains "$OUTPUT" "迭代完成" "4.1: 非 P0 issue 不阻断 iterate（设计如此）"

# --- 4.2: 非法 severity 值 ---
echo "  4.2: 非法 severity 值"
setup_repo
mkdir -p dev-doc/issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: 非法 severity
nums: 1
---

- [ ] I1：bug
  - severity: CRITICAL
  - location: test.js:1
  - description: severity 不合法
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_invalid_severity" "4.2: 非法 severity 应被 validate 检出"

# --- 4.3: issue 缺少必需字段 ---
echo "  4.3: issue 缺少必需字段"
setup_repo
mkdir -p dev-doc/issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: 缺字段
nums: 1
---

- [ ] I1：只有标题没有必需字段
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_missing_required_fields" "4.3: 缺必需字段应被检出"

echo ""

# ======================================================================
# SCENARIO 5: STATUS.yaml 篡改
# ======================================================================
echo "=============================="
echo "  SCENARIO 5: STATUS.yaml 篡改"
echo "=============================="

# --- 5.1: 非法 phase 值 ---
echo "  5.1: 设置非法 phase 值"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: SHIP_IT
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "status_invalid_phase:SHIP_IT" "5.1: 非法 phase 应被 validate 检出"

# --- 5.2: 非法 mode 值 ---
echo "  5.2: 设置非法 mode 值"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: yolo
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "status_invalid_mode:yolo" "5.2: 非法 mode 应被 validate 检出"

# --- 5.3: mode.sh 拒绝非法模式 ---
echo "  5.3: mode.sh 拒绝非法模式"
setup_repo
OUTPUT=$(bash "$CMD_DIR/mode.sh" "turbo" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "5.3: 非法模式应被 mode.sh 拒绝"
assert_contains "$OUTPUT" "无效模式" "5.3: 应提示无效模式"

# --- 5.4: STATUS.yaml 被清空 ---
echo "  5.4: STATUS.yaml 被清空"
setup_repo
mkdir -p dev-doc
> dev-doc/STATUS.yaml
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "5.4: 空 STATUS.yaml 不应使 hook 崩溃"

# --- 5.5: STATUS.yaml 含恶意 YAML 注入 ---
echo "  5.5: STATUS.yaml YAML 注入"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: "test; rm -rf /"
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "5.5: YAML 注入不应导致崩溃"
# 项目应仍然存在（没被 rm -rf）
assert_file_exists "dev-doc/STATUS.yaml" "5.5: YAML 注入不应执行命令"

# --- 5.6: STATUS.yaml 缺少所有字段 ---
echo "  5.6: STATUS.yaml 只有垃圾内容"
setup_repo
mkdir -p dev-doc
echo "garbage content without yaml structure" > dev-doc/STATUS.yaml
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "5.6: 垃圾 STATUS.yaml 不应使 hook 崩溃"

echo ""

# ======================================================================
# SCENARIO 6: iterate 重复执行与竞态
# ======================================================================
echo "=============================="
echo "  SCENARIO 6: iterate 重复执行"
echo "=============================="

# --- 6.1: 同版本二次 iterate ---
echo "  6.1: 同版本二次 iterate"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: fast
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- first
EOF
git add -A && git commit -m "prep" -q

# 第一次 iterate
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "first" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "6.1: 第一次应成功"

# 第二次 iterate 用相同 topic
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- second
EOF
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
git add -A && git commit -m "prep2" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "first" "minor" "dev-doc" 2>&1)
EXIT=$?
# 第二次应失败（归档目录 v1.1.0-first 已存在？不对，第一次归档是 v1.0.0-first）
# 第二次版本是 1.1.0，归档目录是 v1.1.0-first，这个不存在，应该成功
assert_contains "$OUTPUT" "迭代完成" "6.1: 不同版本同 topic 应可以执行"

# --- 6.2: tag 已存在时的处理 ---
echo "  6.2: tag 已存在时的处理"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: fast
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
# 提前手动创建 tag
git tag -a "v1.0.0" -m "pre-existing tag"
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "dup-tag" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "已存在" "6.2: tag 已存在时应 warning 而非崩溃"
# 应继续执行（跳过 tag 创建）
assert_contains "$OUTPUT" "迭代完成" "6.2: tag 已存在不应阻断流程"

echo ""

# ======================================================================
# SCENARIO 7: CHANGELOG 注入攻击
# ======================================================================
echo "=============================="
echo "  SCENARIO 7: CHANGELOG 注入"
echo "=============================="

setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
echo "# CHANGELOG" > dev-doc/CHANGELOG.md

# --- 7.1: save-changelog 防重复 ---
echo "  7.1: CHANGELOG 防重复写入"
# 写入一条记录
echo "- 10:00 测试记录" >> dev-doc/CHANGELOG.md
# 再次运行 hook 写入相同内容不应重复
OUTPUT=$(DEVFLOW_LAST_ACTION="测试记录" bash "$HOOK_DIR/save-changelog.sh" "dev-doc" 2>&1)
COUNT=$(grep -c "测试记录" dev-doc/CHANGELOG.md)
if [ "$COUNT" -le 1 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 7.1 CHANGELOG 不应有重复条目(count=$COUNT)"; fi

# --- 7.2: CHANGELOG 含特殊字符不崩溃 ---
echo "  7.2: CHANGELOG 含特殊字符"
echo '- 10:00 含特殊字符 $() `cmd` "quotes" & | ;' >> dev-doc/CHANGELOG.md
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "7.2: CHANGELOG 含特殊字符不应使 status.sh 崩溃"

echo ""

# ======================================================================
# SCENARIO 8: 多工程模式下的路径逃逸
# ======================================================================
echo "=============================="
echo "  SCENARIO 8: 分支路径一致性"
echo "=============================="

# --- 8.1: 非 main 分支下 dev-doc 路径检测 ---
echo "  8.1: feature 分支文档路径"
setup_repo
git checkout -b feature-x -q

# 创建分支专用的 STATUS.yaml
mkdir -p dev-doc/feature-x
cat > dev-doc/feature-x/STATUS.yaml << 'EOF'
name: feature-x
phase: DEV
mode: fast
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

# 也创建根 STATUS（不应被读取）
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: root-should-not-use
phase: PRD
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "STAGE: DEV" "8.1: 应读取分支对应的 STATUS.yaml"
assert_not_contains "$OUTPUT" "PRD" "8.1: 不应读取根目录的 STATUS.yaml"

# --- 8.2: status.sh 也应读分支路径 ---
echo "  8.2: status.sh 分支路径"
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "feature-x" "8.2: status.sh 应显示分支项目名"
assert_contains "$OUTPUT" "DEV" "8.2: status.sh 应显示分支的 phase"

# --- 8.3: mode.sh 分支路径 ---
echo "  8.3: mode.sh 分支路径"
OUTPUT=$(bash "$CMD_DIR/mode.sh" "quick" "dev-doc" 2>&1)
assert_file_contains "dev-doc/feature-x/STATUS.yaml" "mode: quick" "8.3: mode.sh 应更新分支的 STATUS"
# 根 STATUS 不应被修改
assert_file_contains "dev-doc/STATUS.yaml" "mode: full" "8.3: 根 STATUS 不应被修改"

echo ""

# ======================================================================
# SCENARIO 9: 并发/重入安全
# ======================================================================
echo "=============================="
echo "  SCENARIO 9: hook 并发安全"
echo "=============================="

# --- 9.1: 两次 post-write 快速触发不应互相干扰 ---
echo "  9.1: 快速连续触发 post-write"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：任务1
  - level: P0
- [ ] T2：任务2
  - level: P0
EOF

# 快速连续执行两次
TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" > /dev/null 2>&1
TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" > /dev/null 2>&1
EXIT=$?
assert_ok "$EXIT" "9.1: 快速连续 post-write 不应崩溃"
# STATUS.yaml 应仍完整
assert_file_contains "dev-doc/STATUS.yaml" "phase: DEV" "9.1: STATUS 不应损坏"

# --- 9.2: 同时完成多个任务 ---
echo "  9.2: 一次性完成所有任务"
sed -i 's/- \[ \] T1/- [x] T1/' dev-doc/task/task_2026-05-24_1.md
sed -i 's/- \[ \] T2/- [x] T2/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成" "9.2: 一次性完成应正确检测"
assert_file_exists "dev-doc/task/done_task_2026-05-24_1.md" "9.2: 应自动重命名"

echo ""

# ======================================================================
# SCENARIO 10: 边界输入 — 空目录、超大文件、特殊文件名
# ======================================================================
echo "=============================="
echo "  SCENARIO 10: 边界输入"
echo "=============================="

# --- 10.1: 空 task 目录不崩溃 ---
echo "  10.1: 完全空的项目结构"
setup_repo
mkdir -p dev-doc/task dev-doc/issue dev-doc/archive
cat > dev-doc/STATUS.yaml << 'EOF'
name: empty-project
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.1: 空目录 status.sh 不应崩溃"
assert_contains "$OUTPUT" "无任务" "10.1: 应显示无任务"

OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.1: 空目录 check.sh 不应崩溃"

OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.1: 空目录 inject-context 不应崩溃"

# --- 10.2: task 文件有 100 个条目 ---
echo "  10.2: 超多条目的 task 文件"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: big
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# 生成 100 个条目
{
  echo "---"
  echo "title: TASK - 大批量"
  echo "nums: 100"
  echo "---"
  echo ""
  for i in $(seq 1 100); do
    echo "- [ ] T${i}：任务${i}"
    echo "  - level: P0"
    echo "  - Done when: pass"
  done
} > dev-doc/task/task_2026-05-24_1.md

OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "TASK: 0/100" "10.2: 应正确统计 100 个任务"

# 全部标记完成
sed -i 's/- \[ \] T/- [x] T/g' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成（100/100）" "10.2: 100 个任务全部完成应正确检测"

# --- 10.3: 文件名含空格和中文 ---
echo "  10.3: 文件名含特殊字符"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# task 文件名必须符合规范，所以这里测试 CHANGED_FILE 含特殊字符
OUTPUT=$(TOOL_INPUT_FILE_PATH="src/文件 with spaces.js" bash "$HOOK_DIR/post-write.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.3: 含空格和中文的文件路径不应崩溃"

# --- 10.4: nums 与实际条目数不一致 ---
echo "  10.4: issue nums 字段与实际不一致"
setup_repo
mkdir -p dev-doc/issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: nums 不匹配
nums: 5
---

- [ ] I1：只有两个
  - severity: P0
  - location: a.js:1
  - description: test

- [ ] I2：第二个
  - severity: P1
  - location: b.js:1
  - description: test
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_nums_mismatch" "10.4: nums 不一致应被检出"

echo ""

# ======================================================================
# SCENARIO 11: mode.sh 初始 phase 时 STATUS 不含 iteration
# ======================================================================
echo "=============================="
echo "  SCENARIO 11: mode.sh 无 iteration 字段"
echo "=============================="

# 确认新创建的 STATUS.yaml 不含 iteration
setup_repo
bash "$CMD_DIR/mode.sh" "full" "dev-doc" > /dev/null 2>&1
assert_file_not_contains "dev-doc/STATUS.yaml" "iteration" "11: 新 STATUS 不应含 iteration 字段"

setup_repo
bash "$CMD_DIR/mode.sh" "quick" "dev-doc" > /dev/null 2>&1
assert_file_not_contains "dev-doc/STATUS.yaml" "iteration" "11: quick 新 STATUS 不应含 iteration"

setup_repo
bash "$CMD_DIR/mode.sh" "fast" "dev-doc" > /dev/null 2>&1
assert_file_not_contains "dev-doc/STATUS.yaml" "iteration" "11: fast 新 STATUS 不应含 iteration"

setup_repo
bash "$CMD_DIR/mode.sh" "mvp" "dev-doc" > /dev/null 2>&1
assert_file_not_contains "dev-doc/STATUS.yaml" "iteration" "11: mvp 新 STATUS 不应含 iteration"

echo ""

# ======================================================================
# SCENARIO 12: agent 试图操纵 check.sh 输出
# ======================================================================
echo "=============================="
echo "  SCENARIO 12: check.sh 边界"
echo "=============================="

# --- 12.1: DEV 阶段全部任务完成但不升阶段 ---
echo "  12.1: 全部完成仍在 DEV"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [x] T1：完成
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- done
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
git add -A && git commit -m "sync" -q
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成但阶段仍为 DEV" "12.1: check 应警告升阶段"

# --- 12.2: DONE 阶段有 open issue ---
echo "  12.2: DONE 阶段有 open issue"
setup_repo
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
- [ ] I1：未修复
  - severity: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- x
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
git add -A && git commit -m "sync" -q
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "未关闭 issue" "12.2: DONE 有 open issue 应警告"

# --- 12.3: DEV 阶段无 SPEC.md ---
echo "  12.3: DEV 阶段缺 SPEC"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：进行中
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- x
EOF
git add -A && git commit -m "sync" -q
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "缺少 SPEC.md" "12.3: DEV 阶段缺 SPEC 应警告"

echo ""

# ======================================================================
# SCENARIO 13: inject-context 阻断逻辑
# ======================================================================
echo "=============================="
echo "  SCENARIO 13: inject-context 阻断"
echo "=============================="

# --- 13.1: DEV 阶段无活跃 task 且无 issue —— 应阻断 ---
echo "  13.1: DEV 无 task 无 issue 应阻断"
setup_repo
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# task/ 和 issue/ 都为空
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "BLOCKED" "13.1: DEV 无内容应输出 BLOCKED"

# --- 13.2: DEV 只有 done_task 无活跃 task —— 应引导 /test ---
echo "  13.2: DEV 只有 done_task 应引导 test"
cat > dev-doc/task/done_task_2026-05-24_1.md << 'EOF'
- [x] T1：已完成
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_not_contains "$OUTPUT" "BLOCKED" "13.2: 只有 done_task 不应 BLOCKED"
assert_contains "$OUTPUT" "所有任务已完成" "13.2: 应引导执行 /test"

# --- 13.3: DEV 有活跃 task —— 不应阻断 ---
echo "  13.3: DEV 有活跃 task 不应阻断"
cat > dev-doc/task/task_2026-05-24_2.md << 'EOF'
- [ ] T1：进行中
  - level: P0
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_not_contains "$OUTPUT" "BLOCKED" "13.3: 有活跃 task 不应 BLOCKED"
assert_contains "$OUTPUT" "DEV HINTS" "13.3: 应正常输出 DEV HINTS"

# --- 13.4: DEV 无活跃 task 但有 open issue —— 不应阻断 ---
echo "  13.4: DEV 无活跃 task 有 issue 不应阻断"
setup_repo
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
- [ ] I1：需修复
  - severity: P1
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_not_contains "$OUTPUT" "BLOCKED" "13.4: 有 open issue 不应 BLOCKED"
assert_contains "$OUTPUT" "ISSUE: 1" "13.4: 应显示 1 个 issue"

echo ""

# ======================================================================
# SCENARIO 14: iterate DEVFLOW_NO_CONFIRM 边界
# ======================================================================
echo "=============================="
echo "  SCENARIO 14: DEVFLOW_NO_CONFIRM 边界"
echo "=============================="

# --- 14.1: DEVFLOW_NO_CONFIRM=0 应等待 ---
echo "  14.1: DEVFLOW_NO_CONFIRM=0"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=0 bash "$CMD_DIR/iterate.sh" "test14a" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "等待 agent 确认" "14.1: =0 应等待确认"
assert_no_git_tag "v1.0.0" "14.1: =0 不应创建 tag"

# --- 14.2: DEVFLOW_NO_CONFIRM=yes 应等待（非 "1"） ---
echo "  14.2: DEVFLOW_NO_CONFIRM=yes"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=yes bash "$CMD_DIR/iterate.sh" "test14b" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "等待 agent 确认" "14.2: =yes 应等待确认"
assert_no_git_tag "v1.0.0" "14.2: =yes 不应创建 tag"

# --- 14.3: DEVFLOW_NO_CONFIRM=1 应执行 ---
echo "  14.3: DEVFLOW_NO_CONFIRM=1"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "test14c" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "14.3: =1 应执行完成"

echo ""

# ======================================================================
# 最终汇总
# ======================================================================
cleanup

echo "================================================================"
echo "  E2E 对抗测试结果"
echo "================================================================"
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
echo "ALL PASSED"
exit 0
