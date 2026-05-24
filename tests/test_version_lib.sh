#!/bin/bash
# 测试 scripts/lib/version.sh 版本操作函数库
# 覆盖：T1 - VERSION 文件 + 版本操作函数库

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$PROJECT_ROOT/tmp/test_version_lib_$$"

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

assert_exit() {
  local test_name="$1" expected_code="$2"
  shift 2
  "$@" >/dev/null 2>&1
  local actual_code=$?
  if [ "$expected_code" -eq "$actual_code" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: $test_name\n  expected exit: $expected_code\n  actual exit:   $actual_code"
  fi
}

# === 设置测试环境 ===
setup() {
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  cd "$TMP_DIR"
  git init -q
  git config user.email "test@test.com"
  git config user.name "Test"
  echo "1.0.0" > VERSION
  git add . && git commit -q -m "init"
}

cleanup() {
  cd "$PROJECT_ROOT"
  rm -rf "$TMP_DIR"
}

# === T1: VERSION 文件存在且内容正确 ===
test_version_file_exists() {
  assert_eq "VERSION文件存在" "2.2.0" "$(cat "$PROJECT_ROOT/VERSION" | tr -d '[:space:]')"
}

# === T1: version_read 正常读取 ===
test_version_read_normal() {
  setup
  echo "2.2.0" > "$TMP_DIR/VERSION"
  local result
  result=$(version_read "$TMP_DIR/VERSION")
  assert_eq "version_read正常读取" "2.2.0" "$result"
  cleanup
}

# === T1: version_read 文件不存在返回空 ===
test_version_read_missing_file() {
  local result
  result=$(version_read "/nonexistent/path/VERSION")
  assert_eq "version_read文件不存在返回空" "" "$result"
}

# === T1: version_read 空文件返回空 ===
test_version_read_empty_file() {
  setup
  echo "" > "$TMP_DIR/VERSION"
  local result
  result=$(version_read "$TMP_DIR/VERSION")
  assert_eq "version_read空文件返回空" "" "$result"
  cleanup
}

# === T1: version_read 带尾部换行和空格处理 ===
test_version_read_trailing_whitespace() {
  setup
  printf "2.2.0  \n\n" > "$TMP_DIR/VERSION"
  local result
  result=$(version_read "$TMP_DIR/VERSION")
  assert_eq "version_read去除尾部空白" "2.2.0" "$result"
  cleanup
}

# === T1: version_validate 合法格式 ===
test_version_validate_valid() {
  assert_exit "validate 1.2.3 合法" 0 version_validate "1.2.3"
  assert_exit "validate 0.0.0 合法" 0 version_validate "0.0.0"
  assert_exit "validate 10.20.30 合法" 0 version_validate "10.20.30"
  assert_exit "validate 999.999.999 合法" 0 version_validate "999.999.999"
}

# === T1: version_validate 非法格式 ===
test_version_validate_invalid() {
  assert_exit "validate abc 非法" 1 version_validate "abc"
  assert_exit "validate 空字符串 非法" 1 version_validate ""
  assert_exit "validate v1.2.3 带前缀非法" 1 version_validate "v1.2.3"
  assert_exit "validate 1.2 两段非法" 1 version_validate "1.2"
  assert_exit "validate 1.2.3.4 四段非法" 1 version_validate "1.2.3.4"
  assert_exit "validate 1.2.a 含字母非法" 1 version_validate "1.2.a"
  assert_exit "validate -1.2.3 负数非法" 1 version_validate "-1.2.3"
  assert_exit "validate 1.2.3-rc1 后缀非法" 1 version_validate "1.2.3-rc1"
  assert_exit "validate '1 .2.3' 含空格非法" 1 version_validate "1 .2.3"
}

# === T1: version_bump minor ===
test_version_bump_minor() {
  local result
  result=$(version_bump "2.2.0" minor)
  assert_eq "bump minor 2.2.0→2.3.0" "2.3.0" "$result"
}

# === T1: version_bump major ===
test_version_bump_major() {
  local result
  result=$(version_bump "2.2.0" major)
  assert_eq "bump major 2.2.0→3.0.0" "3.0.0" "$result"
}

# === T1: version_bump patch ===
test_version_bump_patch() {
  local result
  result=$(version_bump "2.2.0" patch)
  assert_eq "bump patch 2.2.0→2.2.1" "2.2.1" "$result"
}

# === T1: version_bump 默认类型 minor ===
test_version_bump_default() {
  local result
  result=$(version_bump "1.0.0")
  assert_eq "bump默认minor 1.0.0→1.1.0" "1.1.0" "$result"
}

# === T1: version_bump 非法类型 ===
test_version_bump_invalid_type() {
  local result
  result=$(version_bump "1.0.0" "invalid" 2>/dev/null)
  local code=$?
  assert_eq "bump非法类型返回非0" "1" "$code"
}

# === T1: version_bump 边界值 ===
test_version_bump_boundary() {
  local result
  result=$(version_bump "0.0.0" patch)
  assert_eq "bump patch 0.0.0→0.0.1" "0.0.1" "$result"
  result=$(version_bump "99.99.99" patch)
  assert_eq "bump patch 99.99.99→99.99.100" "99.99.100" "$result"
}

# === T1: version_write 正常写入 ===
test_version_write_normal() {
  setup
  version_write "3.0.0" "$TMP_DIR/VERSION"
  local result
  result=$(cat "$TMP_DIR/VERSION" | tr -d '[:space:]')
  assert_eq "version_write写入3.0.0" "3.0.0" "$result"
  cleanup
}

# === T1: version_write 校验非法输入 ===
test_version_write_invalid() {
  setup
  version_write "invalid" "$TMP_DIR/VERSION" 2>/dev/null
  local code=$?
  assert_eq "version_write拒绝非法输入" "1" "$code"
  # 确保文件内容未被修改
  local content
  content=$(cat "$TMP_DIR/VERSION" | tr -d '[:space:]')
  assert_eq "version_write非法输入不修改文件" "1.0.0" "$content"
  cleanup
}

# === T1: version_tag_exists 标签不存在 ===
test_version_tag_exists_false() {
  setup
  version_tag_exists "9.9.9"
  local code=$?
  assert_eq "tag不存在返回1" "1" "$code"
  cleanup
}

# === T1: version_tag_exists 标签存在 ===
test_version_tag_exists_true() {
  setup
  git tag -a "v1.0.0" -m "test"
  version_tag_exists "1.0.0"
  local code=$?
  assert_eq "tag存在返回0" "0" "$code"
  cleanup
}

# === T1: version_create_tag 正常创建 ===
test_version_create_tag_normal() {
  setup
  version_create_tag "1.0.0"
  local code=$?
  assert_eq "create_tag成功返回0" "0" "$code"
  # 验证是 annotated tag
  local tag_type
  tag_type=$(git cat-file -t "v1.0.0" 2>/dev/null)
  assert_eq "create_tag为annotated" "tag" "$tag_type"
  # 验证 tag message
  local tag_msg
  tag_msg=$(git tag -l --format='%(contents:subject)' "v1.0.0")
  assert_eq "create_tag消息正确" "Release v1.0.0" "$tag_msg"
  cleanup
}

# === T1: version_create_tag 重复创建应失败 ===
test_version_create_tag_duplicate() {
  setup
  git tag -a "v1.0.0" -m "existing"
  version_create_tag "1.0.0" 2>/dev/null
  local code=$?
  assert_eq "create_tag重复返回1" "1" "$code"
  cleanup
}

# === 运行所有测试 ===
test_version_file_exists
test_version_read_normal
test_version_read_missing_file
test_version_read_empty_file
test_version_read_trailing_whitespace
test_version_validate_valid
test_version_validate_invalid
test_version_bump_minor
test_version_bump_major
test_version_bump_patch
test_version_bump_default
test_version_bump_invalid_type
test_version_bump_boundary
test_version_write_normal
test_version_write_invalid
test_version_tag_exists_false
test_version_tag_exists_true
test_version_create_tag_normal
test_version_create_tag_duplicate

# === 报告 ===
echo ""
echo "=========================="
echo "test_version_lib.sh 结果"
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
