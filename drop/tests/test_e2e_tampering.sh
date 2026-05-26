#!/bin/bash
# E2E 篡改测试：模拟 agent 偷偷修改文件使非法状态看起来合法
# 场景覆盖：删除 issue 逃避阻断、篡改 task 伪造进度、
#   修改 hooks/scripts 绕过检查、伪造文档、隐藏证据等

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CMD_DIR="$SCRIPT_DIR/scripts/commands"
HOOK_DIR="$SCRIPT_DIR/scripts/hooks"
INIT_DIR="$SCRIPT_DIR/scripts/init"
LIB_DIR="$SCRIPT_DIR/scripts/lib"
TMP_DIR="$SCRIPT_DIR/tmp/test_tampering_$$"
PASS=0; FAIL=0; ERRORS=""

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
  if [ "$exit_code" -eq 0 ]; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg (exit=$exit_code)"; fi
}

assert_fail() {
  local exit_code="$1" msg="$2"
  if [ "$exit_code" -ne 0 ]; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg (expected non-zero, got 0)"; fi
}

assert_contains() {
  local output="$1" expected="$2" msg="$3"
  if echo "$output" | grep -qF "$expected"; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg\n    expected: $expected\n    got: $(echo "$output" | head -5)"; fi
}

assert_not_contains() {
  local output="$1" unexpected="$2" msg="$3"
  if ! echo "$output" | grep -qF "$unexpected"; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg\n    should NOT contain: $unexpected"; fi
}

assert_file_exists() {
  local path="$1" msg="$2"
  if [ -f "$path" ]; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg\n    not found: $path"; fi
}

assert_file_not_exists() {
  local path="$1" msg="$2"
  if [ ! -f "$path" ]; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); ERRORS="$ERRORS\n  FAIL: $msg\n    should not exist: $path"; fi
}

assert_file_contains() {
  local path="$1" expected="$2" msg="$3"
  if [ -f "$path" ] && grep -qF "$expected" "$path"; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); local c="(not found)"; [ -f "$path" ] && c=$(head -5 "$path"); ERRORS="$ERRORS\n  FAIL: $msg\n    expected in $path: $expected\n    content: $c"; fi
}

echo "================================================================"
echo "  E2E 篡改测试：agent 偷偷修改文件使状态看似合法"
echo "================================================================"
echo ""

# ======================================================================
# SCENARIO 1: 删除 issue 文件逃避 P0 阻断
# ======================================================================
echo "=============================="
echo "  SCENARIO 1: 删除 issue 文件逃避阻断"
echo "=============================="

# --- 1.1: agent 直接删除 P0 issue 文件 ---
echo "  1.1: 直接删除 P0 issue 文件"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: 严重 bug
nums: 1
---

- [ ] I1：生产环境崩溃
  - severity: P0
  - location: server.js:100
  - description: 内存泄漏导致 OOM
EOF
git add -A && git commit -m "with issue" -q

# agent 偷偷删除 issue 文件
rm dev-doc/issue/issue_test_2026-05-24_1.md
git add -A && git commit -m "cleanup" -q

# iterate 现在不会阻断——文件不在了
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "escaped" "minor" "dev-doc" 2>&1)
# 技术上能通过——但 git log 留下证据
assert_contains "$OUTPUT" "迭代完成" "1.1: 删除 issue 后 iterate 不阻断（已知限制）"
# 验证 git 留有痕迹
EVIDENCE=$(git log --oneline --all | grep -c "cleanup")
if [ "$EVIDENCE" -gt 0 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 1.1 删除操作应留有 git 痕迹"; fi

# --- 1.2: agent 清空 issue 文件内容（不删除） ---
echo "  1.2: 清空 issue 文件内容"
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
---
title: 被清空的 issue
nums: 1
---

- [ ] I1：P0 bug
  - severity: P0
  - location: a.js:1
  - description: critical
EOF
git add -A && git commit -m "with issue" -q

# agent 清空文件但保留文件存在
> dev-doc/issue/issue_test_2026-05-24_1.md
git add -A && git commit -m "update" -q

cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "cleared" "minor" "dev-doc" 2>&1)
# 清空后 grep "severity: P0" 找不到匹配，iterate 不阻断
assert_contains "$OUTPUT" "迭代完成" "1.2: 清空 issue 内容后 iterate 不阻断（已知限制）"

# --- 1.3: agent 把 P0 改为注释 ---
echo "  1.3: 注释掉 severity: P0"
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
- [ ] I1：P0 bug
  - # severity: P0
  - location: a.js:1
  - description: critical
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "commented" "minor" "dev-doc" 2>&1)
# grep "severity: P0" 匹配注释行——所以仍然会阻断！
assert_fail $? "1.3: 注释中的 severity: P0 仍应被检测到（grep 不区分注释）"

echo ""

# ======================================================================
# SCENARIO 2: 篡改 task 文件伪造完成
# ======================================================================
echo "=============================="
echo "  SCENARIO 2: 篡改 task 文件伪造完成"
echo "=============================="

# --- 2.1: 删除未完成条目使 task 看起来全完成 ---
echo "  2.1: 删除未完成条目"
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
---
title: TASK - 测试
nums: 3
---

- [x] T1：已完成
  - level: P0
  - Done when: pass

- [x] T2：已完成
  - level: P0
  - Done when: pass

- [ ] T3：未完成
  - level: P0
  - Done when: pass
EOF
git add -A && git commit -m "original tasks" -q

# agent 偷偷删除 T3
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 测试
nums: 2
---

- [x] T1：已完成
  - level: P0
  - Done when: pass

- [x] T2：已完成
  - level: P0
  - Done when: pass
EOF
git add -A && git commit -m "task update" -q
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "trimmed" "minor" "dev-doc" 2>&1)
# iterate 只看当前状态，不对比历史——通过
assert_contains "$OUTPUT" "迭代完成" "2.1: 删除条目后 iterate 通过（已知限制）"
# 但 git diff 留有证据
DIFF=$(git log -p --all -- dev-doc/task/task_2026-05-24_1.md | grep "^\-.*T3" | head -1)
if [ -n "$DIFF" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 2.1 删除条目应在 git 历史中留有痕迹"; fi

# --- 2.2: nums 与实际条目数不匹配（validate 能检出） ---
echo "  2.2: nums 字段不匹配"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 伪造
nums: 5
---

- [x] T1：唯一完成的任务
  - level: P0
  - Done when: pass
EOF
# validate 只对 issue 检查 nums，不检查 task 的 nums
# 但这暴露了一个设计考量
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
# validate 当前不检查 task nums 匹配——记录为已知限制
assert_not_contains "$OUTPUT" "nums_mismatch" "2.2: validate 当前不检查 task nums（已知限制，仅检查 issue）"

# --- 2.3: agent 全量替换 task 文件为已完成版本 ---
echo "  2.3: 整体替换 task 内容为全完成"
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
# 初始有 5 个复杂任务
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 复杂工程
nums: 5
---

- [ ] T1：重写认证系统
  - level: P0
  - Done when: JWT + OAuth2 全流程通过

- [ ] T2：数据库迁移
  - level: P0
  - Done when: 零停机迁移完成

- [ ] T3：API v2 兼容层
  - level: P0
  - Done when: v1 和 v2 并行工作

- [ ] T4：性能优化
  - level: P1
  - Done when: P99 < 100ms

- [ ] T5：监控告警
  - level: P1
  - Done when: Grafana 面板配置完成
EOF
git add -A && git commit -m "original complex tasks" -q

# agent 偷偷全部标记完成（未实际实现）
sed -i 's/- \[ \]/- [x]/g' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
# hook 会提示全部完成并重命名
assert_contains "$OUTPUT" "所有任务已完成（5/5）" "2.3: 全标 [x] hook 无法区分真假完成"
assert_file_exists "dev-doc/task/done_task_2026-05-24_1.md" "2.3: hook 会自动重命名为 done_"
# I4 修复：批量完成检测应输出警告（5 个 > 2 个阈值）
assert_contains "$OUTPUT" "单次写入标记了" "2.3: I4 批量完成应触发警告"
# git status 能看到未提交的重命名/修改（hook 做了 mv 但未 commit）
UNSTAGED=$(git status --porcelain | wc -l)
if [ "$UNSTAGED" -gt 0 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 2.3 批量操作应产生可追踪的未提交变更"; fi

# --- 2.4: agent 把 Done when 改为必然通过的标准 ---
echo "  2.4: 篡改 Done when 为必然通过标准"
setup_repo
mkdir -p dev-doc/task
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK - 降标
nums: 1
---

- [ ] T1：性能优化
  - level: P0
  - Done when：完成
EOF
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "模糊的 Done when" "2.4: post-write 应检出模糊的 Done when"

echo ""

# ======================================================================
# SCENARIO 3: 篡改 hooks/scripts 绕过检查
# ======================================================================
echo "=============================="
echo "  SCENARIO 3: 篡改 hook 脚本"
echo "=============================="

# --- 3.1: agent 修改 iterate.sh 跳过交付检查 ---
echo "  3.1: 验证原始脚本检查不可通过环境绕过"
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
- [ ] T1：未完成
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q

# 原始脚本不能绕过（即使设置各种环境变量）
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 SKIP_CHECK=1 FORCE=1 bash "$CMD_DIR/iterate.sh" "hack" "minor" "dev-doc" 2>&1)
assert_fail $? "3.1: 原始脚本应阻断未完成 task"
assert_contains "$OUTPUT" "任务未全部完成" "3.1: 应明确提示任务未完成"

# --- 3.2: agent 修改 hooks.json 禁用所有 hook ---
echo "  3.2: 检测 hooks.json 完整性"
# 检查 hooks.json 是否存在且包含关键 hook
assert_file_exists "$SCRIPT_DIR/hooks.json" "3.2: hooks.json 应存在"
assert_file_contains "$SCRIPT_DIR/hooks.json" "inject-context" "3.2: hooks.json 应包含 inject-context"
assert_file_contains "$SCRIPT_DIR/hooks.json" "post-write" "3.2: hooks.json 应包含 post-write"
assert_file_contains "$SCRIPT_DIR/hooks.json" "save-changelog" "3.2: hooks.json 应包含 save-changelog"

# --- 3.3: agent 修改 validate.sh 使其跳过检查 ---
echo "  3.3: 验证 validate.sh 关键检查存在"
# 如果 agent 把 validate.sh 改为空操作，关键 grep 模式会消失
assert_file_contains "$SCRIPT_DIR/scripts/init/validate.sh" "NEEDS_CONFIRM" "3.3: validate 应有确认项逻辑"
assert_file_contains "$SCRIPT_DIR/scripts/init/validate.sh" "WARNINGS" "3.3: validate 应有警告逻辑"
assert_file_contains "$SCRIPT_DIR/scripts/init/validate.sh" "issue_bad_name" "3.3: validate 应检查 issue 命名"
assert_file_contains "$SCRIPT_DIR/scripts/init/validate.sh" "issue_should_be_closed" "3.3: validate 应检查未关闭的完成 issue"

# --- 3.4: agent 修改 version.sh 使 validate 永远返回 true ---
echo "  3.4: 验证 version.sh 校验逻辑存在"
assert_file_contains "$SCRIPT_DIR/scripts/lib/version.sh" "grep -qE" "3.4: version_validate 应有 regex 校验"
assert_file_contains "$SCRIPT_DIR/scripts/lib/version.sh" "^[0-9]" "3.4: version_validate 应校验数字格式"

echo ""

# ======================================================================
# SCENARIO 4: 伪造文档通过阶段检查
# ======================================================================
echo "=============================="
echo "  SCENARIO 4: 伪造文档骗过阶段检查"
echo "=============================="

# --- 4.1: 空壳 PRD 通过 phase-completion 检查 ---
echo "  4.1: 最低限度 PRD 骗过检查"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: PRD
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# 创建满足关键词检查但内容为空的 PRD
cat > dev-doc/PRD.md << 'EOF'
# PRD
## 2. 目标与非目标
### 非目标
无
## 6. 成功指标
TBD
### Must Have
- [x] 无
EOF
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/PRD.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
# phase-completion 用关键词匹配，最低限度能通过
assert_not_contains "$OUTPUT" "缺少" "4.1: 满足关键词的最低 PRD 能通过检查（已知限制）"

# --- 4.2: 空壳 SPEC 通过检查 ---
echo "  4.2: 最低限度 SPEC 骗过检查"
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
## 2. 架构设计
TBD
## 3. 技术选型
选 X，理由：快
## 4. 数据模型
无
EOF
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/SPEC.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_not_contains "$OUTPUT" "缺少" "4.2: 满足关键词的最低 SPEC 能通过检查（已知限制）"

# --- 4.3: SPEC 完全缺少章节应被检出 ---
echo "  4.3: SPEC 缺少关键章节"
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
这是一个空壳文档，没有任何有用信息。
EOF
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/SPEC.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "架构设计" "4.3: 缺架构设计应被检出"
assert_contains "$OUTPUT" "技术选型" "4.3: 缺技术选型应被检出"
assert_contains "$OUTPUT" "数据模型" "4.3: 缺数据模型应被检出"

# --- 4.4: task 文件无 Done when 应被检出 ---
echo "  4.4: task 无 Done when"
mkdir -p dev-doc/task
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
---
title: TASK
nums: 2
---

- [ ] T1：第一个任务
  - level: P0
  - details: 做点什么

- [ ] T2：第二个任务
  - level: P1
  - details: 再做点什么
EOF
sed -i "s/^phase: .*/phase: DEV/" dev-doc/STATUS.yaml
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
assert_contains "$OUTPUT" "Done when" "4.4: 缺 Done when 应被检出"

echo ""

# ======================================================================
# SCENARIO 5: 隐藏开发活动痕迹
# ======================================================================
echo "=============================="
echo "  SCENARIO 5: 隐藏开发活动痕迹"
echo "=============================="

# --- 5.1: agent 清空 CHANGELOG 隐藏记录 ---
echo "  5.1: 清空 CHANGELOG"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG

## 2026-05-24
- 15:00 删除了安全检查
- 14:00 修改了权限文件
- 13:00 关闭了审计日志
EOF
git add -A && git commit -m "activity logged" -q

# agent 清空 CHANGELOG
echo "# CHANGELOG" > dev-doc/CHANGELOG.md

# check.sh 应能检测到 CHANGELOG 只有标题
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
# check.sh 检查 CHANGELOG 非空——但只有标题行也算"非空"
# 但 git diff 留有记录
DIFF_LINES=$(git diff -- dev-doc/CHANGELOG.md | grep "^-.*删除\|^-.*修改\|^-.*关闭" | wc -l)
if [ "$DIFF_LINES" -ge 3 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 5.1 CHANGELOG 清空应在 git diff 中可见"; fi

# --- 5.2: agent 篡改 STATUS.yaml 时间戳假装最近有更新 ---
echo "  5.2: 篡改 STATUS 时间戳"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2020-01-01 00:00
started: 2020-01-01 00:00
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：进行中
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- x
EOF
git add -A && git commit -m "old state" -q

# agent 伪造时间戳为未来
sed -i "s/^updated:.*/updated: 2030-12-31 23:59/" dev-doc/STATUS.yaml
OUTPUT=$(bash "$CMD_DIR/check.sh" "dev-doc" 2>&1)
# check.sh 比较 commit time vs STATUS time
# 伪造的未来时间不会触发 "文档落后代码" 的警告
assert_not_contains "$OUTPUT" "未同步" "5.2: 伪造未来时间戳可骗过同步检查（已知限制）"

# --- 5.3: agent 试图 amend 最近 commit 隐藏 diff ---
echo "  5.3: git amend 隐藏历史（应留 reflog）"
setup_repo
mkdir -p dev-doc
echo "sensitive change" > dev-doc/secret.md
git add -A && git commit -m "add secret" -q
FIRST_SHA=$(git rev-parse HEAD)

# agent amend 替换内容
echo "innocent content" > dev-doc/secret.md
git add -A && git commit --amend -m "innocent change" -q

# commit SHA 变了
SECOND_SHA=$(git rev-parse HEAD)
if [ "$FIRST_SHA" != "$SECOND_SHA" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 5.3 amend 应改变 SHA"; fi

# reflog 保留原始 commit message
REFLOG=$(git reflog | grep "add secret" | head -1)
if [ -n "$REFLOG" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 5.3 reflog 应保留原始 commit 记录"; fi

echo ""

# ======================================================================
# SCENARIO 6: 利用 glob 盲区逃逸
# ======================================================================
echo "=============================="
echo "  SCENARIO 6: 利用 glob/regex 盲区"
echo "=============================="

# --- 6.1: issue 文件名绕过 issue_*.md glob ---
echo "  6.1: 不匹配 glob 的 issue 文件"
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
# 用非标准命名存放 P0 issue，绕过 iterate 的 glob
cat > dev-doc/issue/p0_critical_bug.md << 'EOF'
- [ ] 严重 bug
  - severity: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "glob-escape" "minor" "dev-doc" 2>&1)
# iterate 只扫描 issue_*.md，非标准命名不会被检测
assert_contains "$OUTPUT" "迭代完成" "6.1: 非标准命名 issue 绕过 iterate 检查（已知限制）"

# 但 validate.sh 会检出非标准命名
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "issue_bad_name:p0_critical_bug.md" "6.1: validate 应检出非标准命名"

# --- 6.2: task 文件用 done_ 开头但不含 task_ ---
echo "  6.2: 伪造 done_xxx 文件名"
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
# 标准 done_task_* 命名
cat > dev-doc/task/done_task_2026-05-24_1.md << 'EOF'
- [x] T1：已完成
EOF
# 非标准但包含未完成条目——iterate 不检查这个
cat > dev-doc/task/done_notes_2026-05-24.md << 'EOF'
- [ ] 遗漏的任务
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "glob2" "minor" "dev-doc" 2>&1)
# iterate 只查 done_task_*.md，done_notes_* 不在范围
assert_contains "$OUTPUT" "迭代完成" "6.2: done_notes 不会被 iterate 扫描（已知限制）"

# --- 6.3: 在 task 目录外隐藏 task 文件 ---
echo "  6.3: task/ 目录外的任务文件"
setup_repo
echo "1.0.0" > VERSION
mkdir -p dev-doc/task dev-doc/hidden
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DONE
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# 在 hidden/ 目录放未完成任务（不在 task/ 中）
cat > dev-doc/hidden/task_2026-05-24_1.md << 'EOF'
- [ ] T1：被隐藏的未完成任务
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "hidden" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "6.3: task/ 外的文件不被检查（已知限制）"

echo ""

# ======================================================================
# SCENARIO 7: checkbox 格式操纵
# ======================================================================
echo "=============================="
echo "  SCENARIO 7: checkbox 格式操纵"
echo "=============================="

# --- 7.1: 用不同格式的 checkbox 逃逸检测 ---
echo "  7.1: 非标准 checkbox 格式"
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
# 用 Unicode checkbox 或其他格式
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [x] T1：已完成
  - level: P0
- ☐ T2：用 Unicode 方框表示未完成
  - level: P0
- [X] T3：大写 X 标记
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "unicode" "minor" "dev-doc" 2>&1)
# iterate 用 grep "^- \[" 计数，☐ 和 [X] 的处理：
# "^- \[" 只匹配 "- [" 开头，☐ 不匹配
# "- [X]" 匹配 "^- \[" 但不匹配 "^- \[x\]"（小写 x）
# 所以 TOTAL=2 (T1+T3), DONE=1 (T1)，应阻断
assert_fail $? "7.1: 大写 [X] 不算完成，应阻断"

# --- 7.2: 缩进或前置空格的 checkbox ---
echo "  7.2: 带前置空格的 checkbox"
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
- [x] T1：正常完成
  - level: P0
  - [ ] T2：子任务未完成（缩进）
    - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "indented" "minor" "dev-doc" 2>&1)
# grep "^- \[" 只匹配行首 "- ["，缩进的不匹配
# 所以 TOTAL=1, DONE=1，能通过
assert_contains "$OUTPUT" "迭代完成" "7.2: 缩进 checkbox 不被计入（agent 可利用此隐藏子任务）"

# --- 7.3: 空格变体：-[ ] 不被计入 ---
echo "  7.3: checkbox 空格变体"
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
- [x] T1：正常完成
  - level: P0
-[ ] T2：无空格变体（-[ ] 不匹配 ^- \[）
  - level: P0
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "spacing" "minor" "dev-doc" 2>&1)
# "^- \[" 需要 "- [" 开头；"-[ ]" (无空格) 不匹配
# TOTAL=1, DONE=1，能通过——agent 可利用此变体隐藏未完成条目
assert_contains "$OUTPUT" "迭代完成" "7.3: 缺空格的 checkbox 不被计入（agent 可利用）"

echo ""

# ======================================================================
# SCENARIO 8: 利用 check.sh 的时间比较漏洞
# ======================================================================
echo "=============================="
echo "  SCENARIO 8: 时间戳操纵"
echo "=============================="

# --- 8.1: post-write 修改 dev-doc 文件会更新时间戳 ---
echo "  8.1: dev-doc 文件修改触发时间戳更新"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2020-01-01 00:00
started: 2020-01-01 00:00
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [ ] T1：进行中
  - level: P0
EOF
TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" > /dev/null 2>&1
# updated 时间应被更新为当前时间
UPDATED=$(grep "^updated:" dev-doc/STATUS.yaml | sed 's/^updated: *//')
if [ "$UPDATED" != "2020-01-01 00:00" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 8.1 时间戳应被更新"; fi

# --- 8.2: 非 dev-doc 文件不更新时间戳 ---
echo "  8.2: 非 dev-doc 文件不触发时间戳"
# 重设时间戳
sed -i "s/^updated:.*/updated: 2020-01-01 00:00/" dev-doc/STATUS.yaml
TOOL_INPUT_FILE_PATH="src/main.js" bash "$HOOK_DIR/post-write.sh" > /dev/null 2>&1
UPDATED=$(grep "^updated:" dev-doc/STATUS.yaml | sed 's/^updated: *//')
if [ "$UPDATED" = "2020-01-01 00:00" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 8.2 非 dev-doc 文件不应更新时间戳"; fi

# --- 8.3: CHANGELOG.md 修改不更新时间戳 ---
echo "  8.3: CHANGELOG 修改不触发时间戳"
echo "# CHANGELOG" > dev-doc/CHANGELOG.md
TOOL_INPUT_FILE_PATH="dev-doc/CHANGELOG.md" bash "$HOOK_DIR/post-write.sh" > /dev/null 2>&1
UPDATED=$(grep "^updated:" dev-doc/STATUS.yaml | sed 's/^updated: *//')
if [ "$UPDATED" = "2020-01-01 00:00" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 8.3 CHANGELOG 修改不应更新时间戳"; fi

echo ""

# ======================================================================
# SCENARIO 9: iterate 归档过程中的文件操纵
# ======================================================================
echo "=============================="
echo "  SCENARIO 9: 归档过程操纵"
echo "=============================="

# --- 9.1: PRD/SPEC 在 iterate 前被偷偷替换 ---
echo "  9.1: iterate 前替换文档内容"
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
cat > dev-doc/PRD.md << 'EOF'
# PRD - 原始需求文档
包含完整的需求描述
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC - 原始技术方案
包含完整的技术设计
EOF
git add -A && git commit -m "original docs" -q

# agent 在 iterate 前替换为空壳
echo "# PRD" > dev-doc/PRD.md
echo "# SPEC" > dev-doc/SPEC.md
git add -A && git commit -m "simplify docs" -q

cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "prep" -q
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "replaced" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "9.1: iterate 不检查文档内容质量（已知限制）"

# 归档中的是替换后的内容
assert_file_contains "dev-doc/archive/v1.0.0-replaced/PRD.md" "# PRD" "9.1: 归档的是当前（被替换的）内容"
# 但 git history 有原始内容
ORIGINAL=$(git log -p --all -- dev-doc/PRD.md | grep "包含完整的需求描述")
if [ -n "$ORIGINAL" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 9.1 git 历史应保留原始文档"; fi

# --- 9.2: iterate 后 agent 删除归档目录 ---
echo "  9.2: iterate 后删除归档"
# 接上一个测试（已有归档）
rm -rf dev-doc/archive/v1.0.0-replaced
git add -A && git commit -m "remove archive" -q
# 归档被删了，但 git 有记录
ARCHIVE_EXISTS=$(git log --oneline --all -- dev-doc/archive/v1.0.0-replaced/ | wc -l)
if [ "$ARCHIVE_EXISTS" -gt 0 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 9.2 git 应记录归档目录曾经存在"; fi

echo ""

# ======================================================================
# SCENARIO 10: 测试文件自身的完整性
# ======================================================================
echo "=============================="
echo "  SCENARIO 10: 测试框架完整性"
echo "=============================="

# --- 10.1: test_all.sh 自排除机制 ---
echo "  10.1: test_all.sh 不会递归执行自身"
assert_file_contains "$SCRIPT_DIR/tests/test_all.sh" "test_all.sh" "10.1: test_all.sh 应有自排除逻辑"

# --- 10.2: 所有测试文件符合命名规范 ---
echo "  10.2: 测试文件命名规范"
BAD_TESTS=0
for f in "$SCRIPT_DIR/tests/"*.sh; do
  BASENAME=$(basename "$f")
  if ! echo "$BASENAME" | grep -qE '^test_.*\.sh$'; then
    BAD_TESTS=$((BAD_TESTS + 1))
  fi
done
if [ "$BAD_TESTS" -eq 0 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 10.2 有 $BAD_TESTS 个测试文件不符合 test_*.sh 命名"; fi

# --- 10.3: iterate.sh 的检查不可被环境变量绕过（除 DEVFLOW_NO_CONFIRM） ---
echo "  10.3: 验证 iterate 只认 DEVFLOW_NO_CONFIRM"
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
- [ ] T1：未完成
  - level: P0
EOF
# 尝试各种环境变量欺骗
OUTPUT=$(DEVFLOW_SKIP_CHECK=1 DEVFLOW_FORCE=1 DEVFLOW_NO_VERIFY=1 bash "$CMD_DIR/iterate.sh" "env-hack" "minor" "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "10.3: 非法环境变量不应绕过检查"

# --- 10.4: 脚本路径注入 ---
echo "  10.4: DOC_ROOT 路径注入"
setup_repo
echo "1.0.0" > VERSION
# 尝试路径遍历
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "test" "minor" "../../etc" 2>&1)
EXIT=$?
# 应失败（STATUS.yaml 不存在于该路径）
assert_fail "$EXIT" "10.4: 路径遍历的 DOC_ROOT 应失败"

echo ""

# ======================================================================
# SCENARIO 10.5: guard.sh 独立守卫脚本（I5 验证）
# ======================================================================
echo "=============================="
echo "  SCENARIO 10.5: guard.sh 独立检查"
echo "=============================="

# --- 10.5.1: guard_check_deps 检测依赖违规 ---
echo "  10.5.1: guard 检测依赖违规"
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
- [ ] T1：基础设施
  - level: P0
  - depends on：无
  - Done when：通过

- [x] T2：依赖 T1 的功能
  - level: P0
  - depends on：T1
  - Done when：通过
EOF
OUTPUT=$(source "$LIB_DIR/guard.sh" && guard_check_deps "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "10.5.1: guard 应检测到 T2 依赖 T1 未完成"
assert_contains "$OUTPUT" "依赖违规" "10.5.1: 输出应含依赖违规提示"

# --- 10.5.2: guard_check_deps 无违规时通过 ---
echo "  10.5.2: guard 无违规时通过"
sed -i 's/- \[ \] T1/- [x] T1/' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(source "$LIB_DIR/guard.sh" && guard_check_deps "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.5.2: 依赖满足时 guard 应通过"

# --- 10.5.3: guard_check_batch 检测批量修改 ---
echo "  10.5.3: guard 检测批量完成"
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
- [ ] T2：任务2
- [ ] T3：任务3
- [ ] T4：任务4
EOF
git add -A && git commit -m "tasks" -q
# 批量标记 4 个完成
sed -i 's/- \[ \]/- [x]/g' dev-doc/task/task_2026-05-24_1.md
OUTPUT=$(source "$LIB_DIR/guard.sh" && guard_check_batch "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "10.5.3: 4 个批量完成应触发警告"
assert_contains "$OUTPUT" "单次变更标记了" "10.5.3: 应含批量警告信息"

# --- 10.5.4: guard_check_phase 检测阶段不一致 ---
echo "  10.5.4: guard 检测阶段不一致"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: quick
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# DEV 阶段但无任务文件、无 SPEC
OUTPUT=$(source "$LIB_DIR/guard.sh" && guard_check_phase "dev-doc" 2>&1)
EXIT=$?
assert_fail "$EXIT" "10.5.4: DEV 阶段无任务应报错"
assert_contains "$OUTPUT" "无任务文件" "10.5.4: 应提示无任务"

# --- 10.5.5: guard_check_all 全量检查 ---
echo "  10.5.5: guard_check_all 综合"
setup_repo
mkdir -p dev-doc/task
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
cat > dev-doc/SPEC.md << 'EOF'
# SPEC
EOF
cat > dev-doc/task/task_2026-05-24_1.md << 'EOF'
- [x] T1：已完成
  - level: P0
  - depends on：无
  - Done when：通过
EOF
git add -A && git commit -m "ok state" -q
OUTPUT=$(source "$LIB_DIR/guard.sh" && guard_check_all "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "10.5.5: 合法状态应全部通过"
assert_contains "$OUTPUT" "全部检查通过" "10.5.5: 输出应含通过提示"

echo ""

# ======================================================================
# SCENARIO 11: 多文件联合篡改
# ======================================================================
echo "=============================="
echo "  SCENARIO 11: 多文件联合篡改"
echo "=============================="

# --- 11.1: 同时修改 task + STATUS 试图一致性欺骗 ---
echo "  11.1: task + STATUS 联合篡改"
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
- [ ] T1：复杂任务
  - level: P0
  - Done when: 完整实现并测试

- [ ] T2：另一个任务
  - level: P0
  - Done when: 集成测试通过
EOF
git add -A && git commit -m "real tasks" -q

# agent 联合篡改：标记完成 + 切 phase + 准备 iterate
sed -i 's/- \[ \]/- [x]/g' dev-doc/task/task_2026-05-24_1.md
sed -i "s/^phase: .*/phase: DONE/" dev-doc/STATUS.yaml
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
- 完成了所有任务
EOF
git add -A && git commit -m "all done" -q

# iterate 能通过——因为从数据角度看确实"完成了"
OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "faked" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "11.1: 联合篡改能通过 iterate（已知限制）"

# git 历史应记录这次批量修改
HISTORY=$(git log --oneline | head -5)
if echo "$HISTORY" | grep -q "all done"; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 11.1 git 应记录批量修改 commit"; fi

# --- 11.2: agent 创建假的 done_task 同时删除原 task ---
echo "  11.2: 用假 done_task 替换原 task"
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
# 原始 task 有 10 个复杂条目
{
  echo "---"
  echo "title: TASK - 大工程"
  echo "nums: 10"
  echo "---"
  for i in $(seq 1 10); do
    echo ""
    echo "- [ ] T${i}：复杂任务${i}"
    echo "  - level: P0"
    echo "  - Done when: 完整实现+测试+文档"
  done
} > dev-doc/task/task_2026-05-24_1.md
git add -A && git commit -m "10 tasks" -q

# agent 删除原 task，创建只有 1 个条目的 done_task
rm dev-doc/task/task_2026-05-24_1.md
cat > dev-doc/task/done_task_2026-05-24_1.md << 'EOF'
---
title: TASK - 大工程
nums: 1
---

- [x] T1：唯一任务
  - level: P0
  - Done when: done
EOF
cat > dev-doc/CHANGELOG.md << 'EOF'
# CHANGELOG
EOF
git add -A && git commit -m "completed" -q

OUTPUT=$(DEVFLOW_NO_CONFIRM=1 bash "$CMD_DIR/iterate.sh" "replaced-task" "minor" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "迭代完成" "11.2: 替换 task 能通过 iterate（已知限制）"

# git 历史能看到 10 → 1 的删减
DELETED=$(git log -p -- dev-doc/task/task_2026-05-24_1.md | grep "^\-.*复杂任务" | wc -l)
if [ "$DELETED" -ge 9 ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); ERRORS="$ERRORS\n  FAIL: 11.2 git 应记录被删除的 9 个任务"; fi

echo ""

# ======================================================================
# SCENARIO 12: 利用 sed/grep 特殊字符逃逸
# ======================================================================
echo "=============================="
echo "  SCENARIO 12: 正则表达式逃逸"
echo "=============================="

# --- 12.1: task 条目含 regex 元字符 ---
echo "  12.1: task 条目含 regex 元字符"
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
- [ ] T1：实现 regex .*\.md$ 匹配
  - level: P0
  - Done when: grep -E "^[a-z]+$" 返回正确

- [x] T2：修复 [brackets] 和 (parens)
  - level: P0
  - Done when: pass
EOF
# hook 不应被 regex 元字符干扰
OUTPUT=$(TOOL_INPUT_FILE_PATH="dev-doc/task/task_2026-05-24_1.md" bash "$HOOK_DIR/post-write.sh" 2>&1)
EXIT=$?
assert_ok "$EXIT" "12.1: regex 元字符不应使 hook 崩溃"
assert_contains "$OUTPUT" "任务完成（1/2）" "12.1: 含 regex 字符的条目应被正确计数"

# --- 12.2: issue severity 行含特殊字符 ---
echo "  12.2: severity 行含特殊字符"
setup_repo
mkdir -p dev-doc/issue
cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
title: regex test
nums: 1
---

- [ ] I1：含 $PATH 和 `backtick` 的描述
  - severity: P0
  - location: src/$HOME/test.js:1
  - description: 测试 "quotes" and 'single'
EOF
OUTPUT=$(bash "$INIT_DIR/validate.sh" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "12.2: 含特殊字符的 issue 不应使 validate 崩溃"

# --- 12.3: STATUS.yaml name 含 sed 分隔符 ---
echo "  12.3: STATUS name 含 sed 特殊字符"
setup_repo
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test/project
phase: DEV
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF
# mode.sh 用 sed 更新 mode 字段，name 含 / 不应干扰
OUTPUT=$(bash "$CMD_DIR/mode.sh" "quick" "dev-doc" 2>&1)
EXIT=$?
assert_ok "$EXIT" "12.3: name 含 / 不应干扰 sed 操作"
assert_file_contains "dev-doc/STATUS.yaml" "mode: quick" "12.3: mode 应被正确更新"

echo ""

# ======================================================================
# 最终汇总
# ======================================================================
cleanup

echo "================================================================"
echo "  E2E 篡改测试结果"
echo "================================================================"
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
echo "ALL PASSED"
exit 0
