#!/bin/bash
# 测试 scripts/commands/iterate.sh 迭代命令
# 覆盖：T2 - /iterate 脚本完整流程

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/test_iterate_$$"

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

# 创建一个完整的测试环境
setup_iterate_env() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"

  # 创建基本项目结构
  mkdir -p dev-doc/task dev-doc/issue scripts/lib scripts/commands
  cp "$PROJECT_ROOT/scripts/lib/version.sh" scripts/lib/
  cp "$PROJECT_ROOT/scripts/commands/iterate.sh" scripts/commands/

  # VERSION 文件
  echo "2.2.0" > VERSION

  # STATUS.yaml
  cat > dev-doc/STATUS.yaml << 'EOF'
name: test-project
phase: TEST
mode: full
updated: 2026-05-24 10:00
started: 2026-05-24 10:00
EOF

  # 全部完成的 task 文件
  cat > dev-doc/task/done_task_v2.2.md << 'EOF'
---
title: TASK - 测试任务
nums: 2
---

- [x] T1：任务一
  - level: P0
  - details：测试
  - Done when：完成

- [x] T2：任务二
  - level: P1
  - details：测试
  - Done when：完成
EOF

  # CHANGELOG
  echo "# CHANGELOG" > dev-doc/CHANGELOG.md
  echo "- 测试修改" >> dev-doc/CHANGELOG.md

  # PRD / SPEC
  echo "# PRD" > dev-doc/PRD.md
  echo "# SPEC" > dev-doc/SPEC.md

  git add -A && git commit -q -m "init test env"
}

cleanup() {
  cd "$PROJECT_ROOT"
  rm -rf "$TMP_DIR"
}

# === T2: 无参数打印用法 ===
test_iterate_no_args() {
  setup_iterate_env
  local output
  output=$(bash scripts/commands/iterate.sh 2>&1)
  local code=$?
  assert_eq "无参数退出码1" "1" "$code"
  assert_contains "无参数打印用法" "用法" "$output"
  cleanup
}

# === T2: 任务未完成时阻断 ===
test_iterate_tasks_incomplete() {
  setup_iterate_env
  # 添加未完成的 task
  cat > dev-doc/task/task_v2.2_extra.md << 'EOF'
---
title: TASK
nums: 1
---

- [ ] T3：未完成任务
  - level: P1
  - details：未完成
  - Done when：完成
EOF
  git add -A && git commit -q -m "add incomplete task"

  local output
  output=$(bash scripts/commands/iterate.sh "test-topic" 2>&1)
  local code=$?
  assert_eq "任务未完成退出码1" "1" "$code"
  assert_contains "任务未完成报错" "任务未全部完成" "$output"
  cleanup
}

# === T2: P0 issue 未关闭时阻断 ===
test_iterate_p0_issue_open() {
  setup_iterate_env
  cat > dev-doc/issue/issue_test_2026-05-24_1.md << 'EOF'
---
source: test
nums: 1
---

- [ ] I1：严重问题
  - severity: P0
  - location：test.sh:1
  - description：测试
  - reproduce：执行测试
  - fix：
EOF
  git add -A && git commit -q -m "add P0 issue"

  local output
  output=$(bash scripts/commands/iterate.sh "test-topic" 2>&1)
  local code=$?
  assert_eq "P0 issue未关闭退出码1" "1" "$code"
  assert_contains "P0 issue报错" "P0 issue" "$output"
  cleanup
}

# === T2: VERSION 文件缺失时报错 ===
test_iterate_no_version() {
  setup_iterate_env
  rm VERSION
  git add -A && git commit -q -m "remove VERSION"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test-topic" 2>&1)
  local code=$?
  assert_eq "无VERSION退出码1" "1" "$code"
  assert_contains "无VERSION报错" "VERSION" "$output"
  cleanup
}

# === T2: VERSION 文件格式非法时报错 ===
test_iterate_invalid_version() {
  setup_iterate_env
  echo "invalid" > VERSION
  git add -A && git commit -q -m "invalid version"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "test-topic" 2>&1)
  local code=$?
  assert_eq "非法VERSION退出码1" "1" "$code"
  assert_contains "非法VERSION报错" "格式非法" "$output"
  cleanup
}

# === T2: 无 DEVFLOW_NO_CONFIRM 时展示摘要并停止 ===
test_iterate_no_confirm_shows_summary() {
  setup_iterate_env
  local output
  output=$(bash scripts/commands/iterate.sh "test-topic" 2>&1)
  local code=$?
  assert_eq "无确认停止退出码0" "0" "$code"
  assert_contains "展示当前版本" "v2.2.0" "$output"
  assert_contains "展示归档目录" "archive" "$output"
  assert_contains "等待确认" "确认" "$output"
  cleanup
}

# === T2: 完整流程 - DEVFLOW_NO_CONFIRM=1 ===
test_iterate_full_flow() {
  setup_iterate_env
  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "version-mgmt" "minor" 2>&1)
  local code=$?
  assert_eq "完整流程退出码0" "0" "$code"

  # 验证归档目录被创建
  if [ -d "dev-doc/archive/v2.2.0-version-mgmt" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: 归档目录未创建"
  fi

  # 验证 done_task 被移动到归档
  if [ -f "dev-doc/archive/v2.2.0-version-mgmt/done_task_v2.2.md" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: done_task未归档"
  fi

  # 验证 PRD/SPEC 被复制到归档
  if [ -f "dev-doc/archive/v2.2.0-version-mgmt/PRD.md" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: PRD.md未归档"
  fi

  # 验证 git tag 被创建
  if git tag -l "v2.2.0" | grep -q "v2.2.0"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: git tag v2.2.0 未创建"
  fi

  # 验证 tag 是 annotated
  local tag_type
  tag_type=$(git cat-file -t "v2.2.0" 2>/dev/null)
  assert_eq "tag是annotated" "tag" "$tag_type"

  # 验证 VERSION 已 bump
  local new_ver
  new_ver=$(cat VERSION | tr -d '[:space:]')
  assert_eq "VERSION已bump到2.3.0" "2.3.0" "$new_ver"

  # 验证 STATUS.yaml phase 被重置
  local phase
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')
  assert_eq "phase重置为PRD(full模式)" "PRD" "$phase"

  # 验证 CHANGELOG 被重置
  local cl_content
  cl_content=$(cat dev-doc/CHANGELOG.md)
  assert_eq "CHANGELOG被重置" "# CHANGELOG" "$cl_content"

  # 验证 commit 消息
  local last_msg
  last_msg=$(git log --format=%s -1)
  assert_contains "最后commit消息含新版本" "v2.3.0" "$last_msg"

  local release_msg
  release_msg=$(git log --format=%s -2 | tail -1)
  assert_contains "Release commit消息" "Release v2.2.0" "$release_msg"

  cleanup
}

# === T2: bump_type=major ===
test_iterate_bump_major() {
  setup_iterate_env
  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "major-release" "major" >/dev/null 2>&1

  local new_ver
  new_ver=$(cat VERSION | tr -d '[:space:]')
  assert_eq "major bump 2.2.0→3.0.0" "3.0.0" "$new_ver"
  cleanup
}

# === T2: bump_type=patch ===
test_iterate_bump_patch() {
  setup_iterate_env
  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "hotfix" "patch" >/dev/null 2>&1

  local new_ver
  new_ver=$(cat VERSION | tr -d '[:space:]')
  assert_eq "patch bump 2.2.0→2.2.1" "2.2.1" "$new_ver"
  cleanup
}

# === T2: 归档目录已存在时报错 ===
test_iterate_archive_exists() {
  setup_iterate_env
  mkdir -p "dev-doc/archive/v2.2.0-dup-topic"
  git add -A && git commit -q -m "pre-existing archive"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "dup-topic" 2>&1)
  local code=$?
  assert_eq "归档目录重复退出码1" "1" "$code"
  assert_contains "归档目录已存在报错" "已存在" "$output"
  cleanup
}

# === T2: tag 已存在时跳过创建但不失败 ===
test_iterate_tag_already_exists() {
  setup_iterate_env
  git tag -a "v2.2.0" -m "pre-existing tag"

  local output
  output=$(DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "topic-with-tag" 2>&1)
  local code=$?
  assert_eq "tag已存在不应失败" "0" "$code"
  assert_contains "tag已存在显示警告" "WARNING" "$output"
  cleanup
}

# === T2: phase 按 mode 重置正确性 ===
test_iterate_phase_reset_by_mode() {
  # quick 模式 → SPEC
  setup_iterate_env
  sed -i 's/^mode: .*/mode: quick/' dev-doc/STATUS.yaml
  git add -A && git commit -q -m "quick mode"
  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "quick-test" >/dev/null 2>&1
  local phase
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')
  assert_eq "quick模式重置为SPEC" "SPEC" "$phase"
  cleanup

  # fast 模式 → TASK
  setup_iterate_env
  sed -i 's/^mode: .*/mode: fast/' dev-doc/STATUS.yaml
  git add -A && git commit -q -m "fast mode"
  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "fast-test" >/dev/null 2>&1
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')
  assert_eq "fast模式重置为TASK" "TASK" "$phase"
  cleanup

  # mvp 模式 → SPEC
  setup_iterate_env
  sed -i 's/^mode: .*/mode: mvp/' dev-doc/STATUS.yaml
  git add -A && git commit -q -m "mvp mode"
  DEVFLOW_NO_CONFIRM=1 bash scripts/commands/iterate.sh "mvp-test" >/dev/null 2>&1
  phase=$(grep "^phase:" dev-doc/STATUS.yaml | sed 's/^phase: *//')
  assert_eq "mvp模式重置为SPEC" "SPEC" "$phase"
  cleanup
}

# === 运行所有测试 ===
test_iterate_no_args
test_iterate_tasks_incomplete
test_iterate_p0_issue_open
test_iterate_no_version
test_iterate_invalid_version
test_iterate_no_confirm_shows_summary
test_iterate_full_flow
test_iterate_bump_major
test_iterate_bump_patch
test_iterate_archive_exists
test_iterate_tag_already_exists
test_iterate_phase_reset_by_mode

# === 报告 ===
echo ""
echo "=========================="
echo "test_iterate.sh 结果"
echo "=========================="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
if [ -n "$ERRORS" ]; then
  echo ""
  echo "失败详情："
  echo -e "$ERRORS"
fi
echo ""
exit $FAIL
