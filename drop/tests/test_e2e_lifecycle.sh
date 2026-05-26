#!/bin/bash
# E2E 生命周期测试：模拟完整 dev-flow 流程
# 在隔离的 git 仓库中，模拟 agent 的文件操作，验证从 init 到 iterate 的完整流水线
# 覆盖：正常流程、边界条件、错误恢复

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CMD_DIR="$SCRIPT_DIR/scripts/commands"
HOOK_DIR="$SCRIPT_DIR/scripts/hooks"
INIT_DIR="$SCRIPT_DIR/scripts/init"
LIB_DIR="$SCRIPT_DIR/scripts/lib"
TMP_DIR="$SCRIPT_DIR/tmp/test_e2e_$$"
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

assert_dir_exists() {
  local path="$1" msg="$2"
  if [ -d "$path" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    dir not found: $path"
  fi
}

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    local content="(file not found)"
    [ -f "$path" ] && content=$(cat "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    expected in $path: $expected\n    content: $(echo "$content" | head -5)"
  fi
}

assert_file_not_contains() {
  local path="$1" unexpected="$2" msg="$3"
  if [ -f "$path" ] && ! grep -qF "$unexpected" "$path"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain in $path: $unexpected"
  fi
}

assert_git_tag() {
  local tag="$1" msg="$2"
  if git tag -l "$tag" | grep -q "$tag"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL: $msg\n    tag not found: $tag"
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
echo "  E2E 生命周期测试"
echo "================================================================"
echo ""

# ======================================================================
# SCENARIO 1: 完整 full 模式生命周期（init → dev → test → iterate）
# ======================================================================
echo "=============================="
echo "  SCENARIO 1: full 模式完整生命周期"
echo "=============================="

setup_repo

# --- Step 1: 初始化项目 ---
echo "  Step 1: init + mode"
bash "$CMD_DIR/mode.sh" "full" "dev-doc" > /dev/null 2>&1
assert_file_exists "dev-doc/STATUS.yaml" "S1: mode.sh 应创建 STATUS.yaml"
assert_file_contains "dev-doc/STATUS.yaml" "phase: PRD" "S1: full 模式初始 phase 应为 PRD"
assert_file_contains "dev-doc/STATUS.yaml" "mode: full" "S1: 应设置 mode=full"

# --- Step 2: 创建 VERSION 文件 ---
echo "  Step 2: VERSION 文件"
echo "1.0.0" > VERSION
source "$LIB_DIR/version.sh"
VER=$(version_read)
assert_contains "$VER" "1.0.0" "S1: version_read 应返回 1.0.0"

# --- Step 3: 模拟 PRD → SPEC → TASK 各阶段产出 ---
echo "  Step 3: 模拟文档产出"
cat > dev-doc/PRD.md << 'EOF'
# PRD: 测试项目
## 2. 目标与非目标
### 目标
- 实现用户登录
### 非目标
- 不做第三方登录
## 3. 功能需求
### Must Have
- [ ] 用户名密码登录
## 6. 成功指标
- 登录成功率 > 99%
EOF

cat > dev-doc/SPEC.md << 'EOF'
# SPEC: 测试项目
## 2. 架构设计
MVC 架构
## 3. 技术选型
选择 Express.js，理由：轻量快速
## 4. 数据模型
users 表：id, username, password_hash
EOF

# --- Step 4: 模拟 task 创建（进入 DEV） ---
echo "  Step 4: 创建 task 进入 DEV"
mkdir -p dev-doc/task
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 用户登录
nums: 3
---

- [ ] T1：数据库建表
  - level: P0
  - details：创建 users 表
  - depends on：无
  - Done when：表存在且有正确字段

- [ ] T2：登录接口
  - level: P0
  - details：POST /api/login
  - depends on：T1
  - Done when：返回 JWT token

- [ ] T3：输入校验
  - level: P1
  - details：校验用户名和密码格式
  - depends on：T2
  - Done when：非法输入返回 400
EOF

sed -i "s/^phase: .*/phase: DEV/" dev-doc/STATUS.yaml

# --- Step 5: 验证 inject-context hook ---
echo "  Step 5: inject-context hook"
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "v1.0.0(no-tag)" "S1: hook 应显示版本号和 no-tag 状态"
assert_contains "$OUTPUT" "STAGE: DEV" "S1: hook 应显示 DEV 阶段"
assert_contains "$OUTPUT" "TASK: 0/3" "S1: hook 应显示 0/3 任务进度"
assert_contains "$OUTPUT" "P0 TASK LIST" "S1: hook 应显示 P0 任务列表"

# --- Step 6: 逐步完成任务（模拟 agent 工作） ---
echo "  Step 6: 模拟逐步完成任务"

# 完成 T1
sed -i 's/- \[ \] T1/- [x] T1/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "任务完成（1/3）" "S1: 完成 T1 后应提示 1/3"

# 完成 T2
sed -i 's/- \[ \] T2/- [x] T2/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "任务完成（2/3）" "S1: 完成 T2 后应提示 2/3"

# inject-context 此时应显示 P1 任务（P0 都完成了）
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "TASK: 2/3" "S1: 2 个完成后 hook 应显示 2/3"
assert_contains "$OUTPUT" "P1 TASK LIST" "S1: P0 完成后应显示 P1 任务"

# 完成 T3（最后一个）
sed -i 's/- \[ \] T3/- [x] T3/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成（3/3）" "S1: 全部完成后应提示 3/3"
assert_contains "$OUTPUT" "/test" "S1: 全部完成后应建议 /test"

# task 文件应被自动重命名为 done_
assert_file_exists "dev-doc/task/done_task_2026-05-24_1.md" "S1: 全部完成应自动添加 done_ 前缀"
assert_file_not_exists "dev-doc/task/task_2026-05-24_1.md" "S1: 原 task 文件应被移走"

# --- Step 7: 模拟 TEST 阶段 ---
echo "  Step 7: TEST 阶段 + issue"
sed -i "s/^phase: .*/phase: TEST/" dev-doc/STATUS.yaml

# 模拟 TEST 发现 issue
mkdir -p dev-doc/issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: 登录测试问题
nums: 2
---

- [ ] I1：密码为空时服务端 500
  - severity: P0
  - location: src/auth.js:42
  - description: 空密码未校验导致 crash

- [ ] I2：错误消息暴露内部信息
  - severity: P1
  - location: src/auth.js:55
  - description: 错误响应包含堆栈信息
EOF

# inject-context 有 issue 时应显示 issue 而非 task
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "ISSUE: 2" "S1: hook 应显示 2 个 open issue"
assert_contains "$OUTPUT" "P0 ISSUE LIST" "S1: 应显示 P0 issue 列表"

# --- Step 8: 模拟修复 issue ---
echo "  Step 8: 修复 issue"
sed -i "s/^phase: .*/phase: DEV/" dev-doc/STATUS.yaml

# 修复 P0 issue
sed -i 's/- \[ \] I1/- [x] I1/' dev-doc/issue/issue_test_2026-05-24_1.md

# 修复 P1 issue
sed -i 's/- \[ \] I2/- [x] I2/' dev-doc/issue/issue_test_2026-05-24_1.md

# 模拟 /fix 命令关闭 issue（重命名为 closed_前缀）
# 注：post-write hook 的 issue auto-close 依赖活跃 task 存在才触发
mv dev-doc/issue/issue_test_2026-05-24_1.md dev-doc/issue/closed_issue_test_2026-05-24_1.md
assert_file_exists "dev-doc/issue/closed_issue_test_2026-05-24_1.md" "S1: issue 应被重命名为 closed"
assert_file_not_exists "dev-doc/issue/issue_test_2026-05-24_1.md" "S1: 原 issue 文件应不存在"

# --- Step 9: 进入 DONE 状态，执行 iterate ---
echo "  Step 9: iterate 交付"
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml

# 添加 CHANGELOG 内容
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG

## 2026-05-24
- 15:00 完成用户登录功能
- 14:00 修复空密码 crash
EOF

# 提交所有变更以便 iterate 可以执行
git add -A && git commit -m "complete iteration 1" -q

# 执行 iterate
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "user-login" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S1: iterate 应输出完成"
assert_contains "$OUTPUT" "v1.0.0" "S1: iterate 应显示当前版本"
assert_contains "$OUTPUT" "v1.1.0" "S1: iterate 应显示新版本"

# 验证 git tag
assert_git_tag "v1.0.0" "S1: 应创建 v1.0.0 tag"

# 验证 VERSION bump
assert_file_contains "VERSION" "1.1.0" "S1: VERSION 应 bump 到 1.1.0"

# 验证归档
assert_dir_exists "dev-doc/archive/v1.0.0-user-login" "S1: 归档目录应存在"
assert_file_exists "dev-doc/archive/v1.0.0-user-login/done_task_2026-05-24_1.md" "S1: done_task 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-user-login/issue/closed_issue_test_2026-05-24_1.md" "S1: closed_issue 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-user-login/PRD.md" "S1: PRD 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-user-login/SPEC.md" "S1: SPEC 应被归档"
assert_file_exists "dev-doc/archive/v1.0.0-user-login/CHANGELOG.md" "S1: CHANGELOG 应被归档"

# 原位检查
assert_file_not_exists "dev-doc/task/done_task_2026-05-24_1.md" "S1: done_task 归档后原位不应存在"
assert_file_not_exists "dev-doc/issue/closed_issue_test_2026-05-24_1.md" "S1: closed_issue 归档后原位不应存在"
assert_file_not_exists "dev-doc/PRD.md" "S1: PRD 原位应被移走（mv）"
assert_file_not_exists "dev-doc/SPEC.md" "S1: SPEC 原位应被移走（mv）"

# STATUS 重置
assert_file_contains "dev-doc/STATUS.yaml" "phase: PRD" "S1: full 模式 iterate 后 phase 应重置为 PRD"

# inject-context 验证新状态
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "v1.1.0(no-tag)" "S1: iterate 后 hook 应显示新版本"
assert_contains "$OUTPUT" "STAGE: PRD" "S1: iterate 后 hook 应显示 PRD 阶段"

echo ""

# ======================================================================
# SCENARIO 2: fast 模式（跳过 PRD/SPEC）
# ======================================================================
echo "=============================="
echo "  SCENARIO 2: fast 模式快速迭代"
echo "=============================="

setup_repo

bash "$CMD_DIR/mode.sh" "fast" "dev-doc" > /dev/null 2>&1
assert_file_contains "dev-doc/STATUS.yaml" "phase: TASK" "S2: fast 模式初始 phase 应为 TASK"
echo "1.0.0" > VERSION

# 直接创建 task（无需 PRD/SPEC）
mkdir -p dev-doc/task
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 快速修复
nums: 1
---

- [ ] T1：修复 typo
  - level: P1
  - details：修复文档中的错别字
  - depends on：无
  - Done when：无错别字
EOF

sed -i "s/^phase: .*/phase: DEV/" dev-doc/STATUS.yaml

# 完成任务
sed -i 's/- \[ \] T1/- [x] T1/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "所有任务已完成" "S2: fast 模式任务完成提示"

# iterate
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 修复 typo
EOF
git add -A && git commit -m "fast fix" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "typo-fix" "patch" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S2: fast 模式 iterate 应成功"
assert_file_contains "VERSION" "1.0.1" "S2: patch bump 应为 1.0.1"
assert_file_contains "dev-doc/STATUS.yaml" "phase: TASK" "S2: fast 模式 iterate 后 phase 应重置为 TASK"
assert_git_tag "v1.0.0" "S2: 应创建 v1.0.0 tag"

echo ""

# ======================================================================
# SCENARIO 3: mvp 模式
# ======================================================================
echo "=============================="
echo "  SCENARIO 3: mvp 模式"
echo "=============================="

setup_repo

bash "$CMD_DIR/mode.sh" "mvp" "dev-doc" > /dev/null 2>&1
assert_file_contains "dev-doc/STATUS.yaml" "phase: SPEC" "S3: mvp 模式初始 phase 应为 SPEC"
echo "0.1.0" > VERSION

# mvp 模式直接进 DEV
sed -i "s/^phase: .*/phase: DEV/" dev-doc/STATUS.yaml

# mvp 没有 task，直接 iterate
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/SPEC.md << 'EOF'
# MVP SPEC
快速验证
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- MVP 完成
EOF
git add -A && git commit -m "mvp done" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "mvp-proto" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S3: mvp iterate 应成功"
assert_file_contains "VERSION" "0.2.0" "S3: minor bump 应为 0.2.0"
assert_file_contains "dev-doc/STATUS.yaml" "phase: SPEC" "S3: mvp 模式 iterate 后 phase 应重置为 SPEC"

echo ""

# ======================================================================
# SCENARIO 4: major 版本升级
# ======================================================================
echo "=============================="
echo "  SCENARIO 4: major 版本 bump"
echo "=============================="

setup_repo

bash "$CMD_DIR/mode.sh" "full" "dev-doc" > /dev/null 2>&1
echo "1.5.3" > VERSION
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 大版本重构
EOF
git add -A && git commit -m "major release" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "v2-rewrite" "major" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S4: major bump iterate 应成功"
assert_file_contains "VERSION" "2.0.0" "S4: major bump 应为 2.0.0"
assert_git_tag "v1.5.3" "S4: 应创建 v1.5.3 tag"

echo ""

# ======================================================================
# SCENARIO 5: 边界条件 — iterate 阻断场景
# ======================================================================
echo "=============================="
echo "  SCENARIO 5: iterate 阻断场景"
echo "=============================="

# --- 5.1: 未完成 task 阻断 ---
echo "  5.1: 未完成 task 阻断"
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
- [x] T1：已完成
  - level: P0
- [ ] T2：未完成
  - level: P0
EOF

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "blocked" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "S5.1: 未完成 task 应阻断 iterate"
assert_contains "$OUTPUT" "任务未全部完成" "S5.1: 应提示任务未完成"

# --- 5.2: P0 issue 阻断 ---
echo "  5.2: P0 issue 阻断"
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
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
- [ ] I1：严重 bug
  - severity: P0
EOF

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "blocked" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "S5.2: P0 issue 应阻断 iterate"
assert_contains "$OUTPUT" "P0 issue" "S5.2: 应提示 P0 issue"

# --- 5.3: 非 P0 issue 不阻断 ---
echo "  5.3: 非 P0 issue 不阻断"
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
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
- [ ] I1：小问题
  - severity: P1
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "with-p1" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S5.3: P1 issue 不应阻断 iterate"

# --- 5.4: VERSION 缺失阻断 ---
echo "  5.4: VERSION 缺失阻断"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# 故意不创建 VERSION

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "no-ver" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "S5.4: 无 VERSION 应阻断 iterate"
assert_contains "$OUTPUT" "VERSION" "S5.4: 应提示 VERSION 问题"

# --- 5.5: VERSION 格式非法阻断 ---
echo "  5.5: VERSION 格式非法阻断"
setup_repo
echo "abc" > VERSION
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "bad-ver" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "S5.5: 非法 VERSION 应阻断 iterate"
assert_contains "$OUTPUT" "非法" "S5.5: 应提示格式非法"

# --- 5.6: 归档目录已存在阻断 ---
echo "  5.6: 归档目录已存在阻断"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task dev-doc/archive/v1.0.0-dup
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "dup" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "S5.6: 归档目录已存在应阻断"
assert_contains "$OUTPUT" "归档目录已存在" "S5.6: 应提示目录已存在"

echo ""

# ======================================================================
# SCENARIO 6: 连续多次迭代
# ======================================================================
echo "=============================="
echo "  SCENARIO 6: 连续多次迭代"
echo "=============================="

setup_repo
echo "1.0.0" > VERSION
bash "$CMD_DIR/mode.sh" "fast" "dev-doc" > /dev/null 2>&1

# 第一次迭代
mkdir -p dev-doc/task
cat > dev-doc/task/done_task_2026-05-24_1.md << 'EOF'
- [x] T1：功能A
EOF
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 功能A
EOF
git add -A && git commit -m "iter1" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "feat-a" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S6: 第一次 iterate 应成功"
assert_file_contains "VERSION" "1.1.0" "S6: 第一次 bump 应为 1.1.0"
assert_git_tag "v1.0.0" "S6: 第一次应创建 v1.0.0 tag"

# 第二次迭代
mkdir -p dev-doc/task
cat > dev-doc/task/done_task_2026-05-24_2.md << 'EOF'
- [x] T1：功能B
EOF
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 功能B
EOF
git add -A && git commit -m "iter2" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "feat-b" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S6: 第二次 iterate 应成功"
assert_file_contains "VERSION" "1.2.0" "S6: 第二次 bump 应为 1.2.0"
assert_git_tag "v1.1.0" "S6: 第二次应创建 v1.1.0 tag"

# 第三次迭代 (patch)
mkdir -p dev-doc/task
cat > dev-doc/task/done_task_2026-05-24_3.md << 'EOF'
- [x] T1：修复C
EOF
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 修复C
EOF
git add -A && git commit -m "iter3" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "fix-c" "patch" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "S6: 第三次 iterate 应成功"
assert_file_contains "VERSION" "1.2.1" "S6: 第三次 patch bump 应为 1.2.1"
assert_git_tag "v1.2.0" "S6: 第三次应创建 v1.2.0 tag"

# 验证归档目录共存
assert_dir_exists "dev-doc/archive/v1.0.0-feat-a" "S6: 第一次归档应保留"
assert_dir_exists "dev-doc/archive/v1.1.0-feat-b" "S6: 第二次归档应保留"
assert_dir_exists "dev-doc/archive/v1.2.0-fix-c" "S6: 第三次归档应保留"

echo ""

# ======================================================================
# SCENARIO 7: version.sh 函数库边界测试
# ======================================================================
echo "=============================="
echo "  SCENARIO 7: version.sh 边界测试"
echo "=============================="

setup_repo
source "$LIB_DIR/version.sh"

# 合法版本号
version_validate "0.0.0"; assert_ok $? "S7: 0.0.0 应合法"
version_validate "99.99.99"; assert_ok $? "S7: 99.99.99 应合法"
version_validate "1.0.0"; assert_ok $? "S7: 1.0.0 应合法"

# 非法版本号
version_validate ""; assert_fail $? "S7: 空字符串应非法"
version_validate "abc"; assert_fail $? "S7: abc 应非法"
version_validate "1.0"; assert_fail $? "S7: 1.0 (缺少 patch) 应非法"
version_validate "1.0.0.0"; assert_fail $? "S7: 1.0.0.0 (四段) 应非法"
version_validate "v1.0.0"; assert_fail $? "S7: v1.0.0 (带 v 前缀) 应非法"
version_validate "1.0.0-beta"; assert_fail $? "S7: 1.0.0-beta (带后缀) 应非法"

# bump 边界
RESULT=$(version_bump "0.0.0" "patch")
assert_contains "$RESULT" "0.0.1" "S7: 0.0.0 patch → 0.0.1"
RESULT=$(version_bump "0.0.0" "minor")
assert_contains "$RESULT" "0.1.0" "S7: 0.0.0 minor → 0.1.0"
RESULT=$(version_bump "0.0.0" "major")
assert_contains "$RESULT" "1.0.0" "S7: 0.0.0 major → 1.0.0"
RESULT=$(version_bump "1.9.9" "patch")
assert_contains "$RESULT" "1.9.10" "S7: 1.9.9 patch → 1.9.10"
RESULT=$(version_bump "1.9.9" "minor")
assert_contains "$RESULT" "1.10.0" "S7: 1.9.9 minor → 1.10.0（patch 归零）"
RESULT=$(version_bump "1.9.9" "major")
assert_contains "$RESULT" "2.0.0" "S7: 1.9.9 major → 2.0.0（minor/patch 归零）"

# tag 操作
echo "1.0.0" > VERSION
version_create_tag "1.0.0"
version_tag_exists "1.0.0"; assert_ok $? "S7: 创建后 tag 应存在"
version_tag_exists "2.0.0"; assert_fail $? "S7: 未创建的 tag 不应存在"

# write + read 循环
version_write "3.14.159"
VER=$(version_read)
assert_contains "$VER" "3.14.159" "S7: write + read 应一致"

echo ""

# ======================================================================
# SCENARIO 8: hook 边界条件
# ======================================================================
echo "=============================="
echo "  SCENARIO 8: hook 边界条件"
echo "=============================="

# --- 8.1: dev-doc 不存在时 hook 静默退出 ---
echo "  8.1: 无 dev-doc 时 hook 静默退出"
setup_repo
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "S8.1: 无 dev-doc inject-context 应正常退出"
# 输出应为空（静默）
if [ -z "$OUTPUT" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: S8.1 无 dev-doc 时 inject-context 应无输出"; fi

OUTPUT=$(TOOL_INPUT_FILE_PATH="test.js" bash "$HOOK_DIR/post-write.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "S8.1: 无 dev-doc post-write 应正常退出"

# --- 8.2: STATUS.yaml 无 phase 字段时 ---
echo "  8.2: STATUS.yaml 缺 phase 字段"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: broken
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "S8.2: 缺 phase 时 inject-context 应正常退出不崩溃"

# --- 8.3: 空 task 目录 ---
echo "  8.3: 空 task 目录"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$HOOK_DIR/inject-context.sh" 2>&1)
assert_contains "$OUTPUT" "TASK: 0/0" "S8.3: 空 task 目录应显示 0/0"

# --- 8.4: post-write 非 dev-doc 文件不触发阶段检查 ---
echo "  8.4: 非 dev-doc 文件修改"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
echo "hello" > src.js
OUTPUT=$(TOOL_INPUT_FILE_PATH="src.js" bash "$HOOK_DIR/post-write.sh" 2>&1)
# 不应改变 STATUS.yaml updated（因为改的不是 dev-doc 文件）
assert_file_contains "dev-doc/STATUS.yaml" "updated: 2026-05-24 10:00" "S8.4: 非 dev-doc 修改不应更新时间戳"

# --- 8.5: post-write 对 STATUS.yaml 自身的修改不更新时间戳 ---
echo "  8.5: STATUS.yaml 自身修改"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/STATUS.yaml" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_file_contains "dev-doc/STATUS.yaml" "updated: 2026-05-24 10:00" "S8.5: STATUS.yaml 自身修改不应触发时间戳更新"

echo ""

# ======================================================================
# SCENARIO 9: status.sh 和 check.sh 边界条件
# ======================================================================
echo "=============================="
echo "  SCENARIO 9: status/check 边界"
echo "=============================="

# --- 9.1: status 无 VERSION ---
echo "  9.1: status 无 VERSION"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "未设置" "S9.1: 无 VERSION 应显示未设置"

# --- 9.2: status VERSION 有 tag ---
echo "  9.2: status VERSION 有对应 tag"
setup_repo
mkdir -p dev-doc
echo "2.0.0" > VERSION
git tag -a "v2.0.0" -m "release" 2>/dev/null
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
OUTPUT=$(bash "$CMD_DIR/status.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "v2.0.0" "S9.2: status 应显示版本号"
assert_contains "$OUTPUT" "已同步" "S9.2: 有 tag 时应显示已同步"

# --- 9.3: check 所有正常 ---
echo "  9.3: check 一切正常"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- test entry
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：正在进行
EOF
git add -A && git commit -m "sync" -q
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "正常" "S9.3: 正常状态应有正常提示"

echo ""

# ======================================================================
# SCENARIO 10: 模式切换场景
# ======================================================================
echo "=============================="
echo "  SCENARIO 10: 模式切换"
echo "=============================="

setup_repo

# full → fast 切换
bash "$CMD_DIR/mode.sh" "full" "dev-doc" > /dev/null 2>&1
assert_file_contains "dev-doc/STATUS.yaml" "mode: full" "S10: 初始 full"

bash "$CMD_DIR/mode.sh" "fast" "dev-doc" > /dev/null 2>&1
assert_file_contains "dev-doc/STATUS.yaml" "mode: fast" "S10: 切换到 fast"
# phase 不变（只改 mode）
assert_file_contains "dev-doc/STATUS.yaml" "phase: PRD" "S10: 切换 mode 不应改 phase"

# fast → mvp 切换
bash "$CMD_DIR/mode.sh" "mvp" "dev-doc" > /dev/null 2>&1
assert_file_contains "dev-doc/STATUS.yaml" "mode: mvp" "S10: 切换到 mvp"

echo ""

# ======================================================================
# SCENARIO 11: DEVFLOW_NO_CONFIRM 未设置时 iterate 暂停
# ======================================================================
echo "=============================="
echo "  SCENARIO 11: iterate 交互确认暂停"
echo "=============================="

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
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q

# 不设 DEVFLOW_NO_CONFIRM（默认行为：展示摘要后退出等待确认）
OUTPUT=$(bash "$CMD_DIR/iterate.sh" "confirm-test" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "S11: 无 NO_CONFIRM 时应正常退出（等待确认）"
assert_contains "$OUTPUT" "迭代摘要" "S11: 应展示摘要"
assert_contains "$OUTPUT" "等待 agent 确认" "S11: 应提示等待确认"
# 不应实际执行 commit/tag
assert_no_git_tag "v1.0.0" "S11: 未确认时不应创建 tag"
assert_file_contains "VERSION" "1.0.0" "S11: 未确认时 VERSION 不应变化"

echo ""

# ======================================================================
# SCENARIO 12: validate.sh 在生命周期中的表现
# ======================================================================
echo "=============================="
echo "  SCENARIO 12: validate 集成"
echo "=============================="

setup_repo
mkdir -p dev-doc/task dev-doc/issue

# 有格式问题的 task 文件（命名不符合 task_YYYY-MM-DD_N.md 规范）
cat > dev-doc/task/task_bad_date.md << 'EOF'
- [ ] 不符合命名规范的任务
EOF

# 有格式问题的 issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: test issue
nums: 1
---

- [ ] I1：测试问题
  - severity: P0
  - location: test.js:1
  - description: 测试
EOF

OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "task_bad_name:task_bad_date.md" "S12: 应检测到不规范的 task 文件名"

# 正确命名的文件不应报错
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：正确格式
  Done when: 测试通过
EOF
rm dev-doc/task/task_bad_date.md

OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_not_contains "$OUTPUT" "task_bad_name" "S12: 修正后不应有 task 命名问题"

echo ""

# ======================================================================
# 最终汇总
# ======================================================================
cleanup

echo "================================================================"
echo "  E2E 生命周期测试结果"
echo "================================================================"
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
echo "ALL PASSED"
exit 0
