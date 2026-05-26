#!/bin/bash
# 测试 P0 issue 修复时自动 bump minor
# 覆盖：T8 - P0 issue 修复自动 bump minor
# 注意：T8 是文档级规范（fix.md 中定义的流程），此处测试底层函数支持

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/test_p0_fix_bump_$$"

source "$PROJECT_ROOT/scripts/lib/version.sh"

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

setup() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"
  echo "2.2.0" > VERSION
  git add -A && git commit -q -m "init"
}

cleanup() {
  cd "$PROJECT_ROOT"
  rm -rf "$TMP_DIR"
}

# === T8: 模拟 P0 issue 关闭后 bump minor 流程 ===
# fix.md 定义的流程：关闭 P0 issue → source version.sh → read → bump minor → write → commit
test_p0_fix_bump_simulation() {
  setup

  # 模拟 fix.md 中定义的 P0 关闭流程
  local VER NEW_VER
  VER=$(version_read "$TMP_DIR/VERSION")
  assert_eq "读取当前版本" "2.2.0" "$VER"

  NEW_VER=$(version_bump "$VER" minor)
  assert_eq "bump minor" "2.3.0" "$NEW_VER"

  version_write "$NEW_VER" "$TMP_DIR/VERSION"
  local written
  written=$(cat "$TMP_DIR/VERSION" | tr -d '[:space:]')
  assert_eq "写入新版本" "2.3.0" "$written"

  # 模拟 git commit
  git add VERSION
  git commit -q -m "Bump to v2.3.0: P0 issue fixed"
  local msg
  msg=$(git log --format=%s -1)
  assert_eq "commit消息正确" "Bump to v2.3.0: P0 issue fixed" "$msg"

  cleanup
}

# === T8: 连续两次 P0 修复后 bump 正确性 ===
test_p0_fix_bump_twice() {
  setup

  # 第一次 P0 修复
  local VER NEW_VER
  VER=$(version_read "$TMP_DIR/VERSION")
  NEW_VER=$(version_bump "$VER" minor)
  version_write "$NEW_VER" "$TMP_DIR/VERSION"
  git add VERSION && git commit -q -m "Bump to v${NEW_VER}: P0 fix 1"

  # 第二次 P0 修复
  VER=$(version_read "$TMP_DIR/VERSION")
  assert_eq "第二次读取为2.3.0" "2.3.0" "$VER"
  NEW_VER=$(version_bump "$VER" minor)
  assert_eq "第二次bump为2.4.0" "2.4.0" "$NEW_VER"
  version_write "$NEW_VER" "$TMP_DIR/VERSION"

  local final
  final=$(cat "$TMP_DIR/VERSION" | tr -d '[:space:]')
  assert_eq "最终版本2.4.0" "2.4.0" "$final"

  cleanup
}

# === T8: fix.md 文档中包含 P0 bump 流程定义 ===
test_fix_md_has_p0_bump_doc() {
  local file="$PROJECT_ROOT/commands/fix.md"
  if [ ! -f "$file" ]; then
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: commands/fix.md不存在"
    return
  fi

  local content
  content=$(cat "$file")

  if echo "$content" | grep -q "P0.*bump\|bump.*P0\|P0.*minor"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: fix.md未包含P0 bump规则"
  fi

  if echo "$content" | grep -q "version_bump\|version_write"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: fix.md未引用版本函数库"
  fi
}

# === 运行所有测试 ===
test_p0_fix_bump_simulation
test_p0_fix_bump_twice
test_fix_md_has_p0_bump_doc

# === 报告 ===
echo ""
echo "=========================="
echo "test_p0_fix_bump.sh 结果"
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
