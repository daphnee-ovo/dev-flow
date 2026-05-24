#!/bin/bash
# 测试 iteration 字段的移除
# 覆盖：T5 - 移除 STATUS.yaml 中的 iteration 字段
# 覆盖：T6 - iterate.md 文档
# 覆盖：T7 - 废弃 /done 命令

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

# === T5: scripts/ 下无 iteration 引用（排除 commit 消息字符串） ===
test_no_iteration_in_scripts() {
  local matches
  # 排除 commit message 中的 "iteration" 字符串
  matches=$(grep -rn "iteration" "$PROJECT_ROOT/scripts/" 2>/dev/null | grep -v 'commit.*iteration' | grep -v '^Binary')
  if [ -z "$matches" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: scripts/下仍有iteration引用\n  匹配:\n$matches"
  fi
}

# === T5: STATUS.yaml 无 iteration 字段 ===
test_no_iteration_in_status() {
  local matches
  matches=$(grep "^iteration:" "$PROJECT_ROOT/dev-doc/STATUS.yaml" 2>/dev/null)
  if [ -z "$matches" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: STATUS.yaml仍有iteration字段\n  匹配: $matches"
  fi
}

# === T6: iterate.md 存在且大于 1KB ===
test_iterate_md_exists() {
  local file="$PROJECT_ROOT/commands/iterate.md"
  if [ -f "$file" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: commands/iterate.md不存在"
    return
  fi

  local size
  size=$(wc -c < "$file")
  if [ "$size" -gt 1024 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: commands/iterate.md小于1KB (实际: ${size}B)"
  fi
}

# === T6: iterate.md 包含关键段落 ===
test_iterate_md_content() {
  local file="$PROJECT_ROOT/commands/iterate.md"
  local content
  content=$(cat "$file" 2>/dev/null)

  assert_contains "iterate.md含交付检查" "交付检查" "$content"
  assert_contains "iterate.md含归档" "归档" "$content"
  assert_contains "iterate.md含commit" "commit" "$content"
  assert_contains "iterate.md含tag" "tag" "$content"
  assert_contains "iterate.md含bump" "bump" "$content"
  assert_contains "iterate.md含VERSION" "VERSION" "$content"
}

# === T7: done.md 标记废弃 ===
test_done_deprecated() {
  local file="$PROJECT_ROOT/commands/done.md"
  if [ ! -f "$file" ]; then
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: commands/done.md不存在"
    return
  fi

  local content
  content=$(cat "$file")
  if echo "$content" | grep -qiE "废弃|deprecated"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: done.md未包含废弃标记"
  fi

  # 验证重定向到 /iterate
  if echo "$content" | grep -qF "/iterate"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\nFAIL: done.md未重定向到/iterate"
  fi
}

# === 运行所有测试 ===
test_no_iteration_in_scripts
test_no_iteration_in_status
test_iterate_md_exists
test_iterate_md_content
test_done_deprecated

# === 报告 ===
echo ""
echo "=========================="
echo "test_iteration_field_removal.sh 结果"
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
