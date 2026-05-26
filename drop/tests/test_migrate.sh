#!/bin/bash
# 测试 scripts/init/migrate.sh

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$SCRIPT_DIR/scripts/init/migrate.sh"
TMP_DIR="$SCRIPT_DIR/tmp/test_migrate_$$"
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
    local content=""
    [ -f "$path" ] && content=$(cat "$path")
    ERRORS="$ERRORS\n  FAIL: $msg\n    file: $path\n    expected: $expected\n    content: $content"
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

# === TEST 1: TASK.md → task/ 迁移 + .bak 保留 ===
echo "TEST 1: TASK.md → task/ 迁移"
setup
mkdir -p dev-doc
cat > dev-doc/TASK.md << 'EOF'
# 任务列表

- [ ] 功能A
  Done when: 测试通过
- [ ] 功能B
  Done when: 部署成功
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "migration_performed" "应执行迁移"
assert_contains "$OUTPUT" "TASK.md" "应报告 TASK.md 迁移"
assert_file_exists "dev-doc/TASK.md.bak" "应保留 .bak 备份"
assert_file_not_exists "dev-doc/TASK.md" "原 TASK.md 应被移除"
assert_dir_exists "dev-doc/task" "应创建 task/ 目录"
# 检查迁移后的文件存在
TODAY=$(date +%Y-%m-%d)
assert_file_exists "dev-doc/task/task_${TODAY}_1.md" "应创建 task 文件"
assert_file_contains "dev-doc/task/task_${TODAY}_1.md" "功能A" "迁移文件应含原内容"

# === TEST 2: session/ → CHANGELOG.md 提取 ===
echo "TEST 2: session/ → CHANGELOG.md 提取"
setup
mkdir -p dev-doc/session
echo "# Session 1 内容" > dev-doc/session/01-init.md
echo "# Session 2 内容" > dev-doc/session/02-feature.md
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "migration_performed" "应执行迁移"
assert_file_exists "dev-doc/CHANGELOG.md" "应创建 CHANGELOG.md"
assert_file_contains "dev-doc/CHANGELOG.md" "# CHANGELOG" "应有 CHANGELOG 头部"
assert_file_contains "dev-doc/CHANGELOG.md" "migrated from session" "应有 migrated 标记"
assert_file_contains "dev-doc/CHANGELOG.md" "init" "应包含 session 文件名信息"
# session/ 目录应保留（不删除）
assert_dir_exists "dev-doc/session" "session/ 应保留不删除"

# === TEST 3: session/ 有但 CHANGELOG.md 已存在时跳过 ===
echo "TEST 3: CHANGELOG.md 已存在时跳过 session 迁移"
setup
mkdir -p dev-doc/session
echo "# Session" > dev-doc/session/01-init.md
echo "# 已有 CHANGELOG" > dev-doc/CHANGELOG.md
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "CHANGELOG.md already present" "已有 CHANGELOG 时应跳过"
assert_file_contains "dev-doc/CHANGELOG.md" "已有 CHANGELOG" "不应覆盖已有 CHANGELOG"

# === TEST 4: phase=MVP → DEV 替换 ===
echo "TEST 4: phase=MVP → DEV 替换"
setup
mkdir -p dev-doc
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: MVP
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "MVP → DEV" "应报告 MVP → DEV 迁移"
assert_file_contains "dev-doc/STATUS.yaml" "phase: DEV" "STATUS 应更新为 DEV"

# === TEST 5: 无需迁移时输出 no_migration_needed ===
echo "TEST 5: 无需迁移时输出 no_migration_needed"
setup
mkdir -p dev-doc/task dev-doc/issue
cat > dev-doc/STATUS.yaml << 'EOF'
name: test
phase: DEV
mode: full
iteration: 1
updated: 2026-05-15 10:00
started: 2026-05-15 10:00
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_contains "$OUTPUT" "no_migration_needed" "无需迁移时应输出 no_migration_needed"

# === TEST 6: TASK.md 迁移序号避免覆盖 ===
echo "TEST 6: TASK.md 迁移避免覆盖已有文件"
setup
mkdir -p dev-doc/task
TODAY=$(date +%Y-%m-%d)
echo "# 已有文件" > "dev-doc/task/task_${TODAY}_1.md"
cat > dev-doc/TASK.md << 'EOF'
- [ ] 新任务
  Done when: pass
EOF
OUTPUT=$(bash "$SCRIPT" "dev-doc" 2>&1)
assert_file_exists "dev-doc/task/task_${TODAY}_2.md" "应使用递增序号避免覆盖"
assert_file_contains "dev-doc/task/task_${TODAY}_1.md" "已有文件" "原有文件不应被覆盖"

# === 汇总 ===
teardown
echo ""
echo "=== migrate.sh 测试结果 ==="
echo "PASS: $PASS  FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo -e "$ERRORS"
  exit 1
fi
exit 0
