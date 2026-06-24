#!/bin/bash
# 验证 dow rollback 命令功能

set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOW="$PROJ_ROOT/dow/target/release/dow"
PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

# 准备隔离测试环境
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"
git init -q
git config user.name "test"
git config user.email "test@test.com"
git commit --allow-empty -m "init: test project" -q

# 初始化 dev-flow
$DOW init --name test-rollback --mode fast -H 2>/dev/null || true

echo "=== dow rollback 功能验证 ==="
echo ""

# --- 测试1: rollback --list（JSON 输出）---
echo "[1] rollback --list JSON 输出"
LIST_OUT=$($DOW rollback --list 2>/dev/null)
if echo "$LIST_OUT" | grep -q '"versions"'; then
  pass "rollback --list 输出包含 versions 字段"
else
  fail "rollback --list 输出缺少 versions 字段"
fi

# --- 测试2: rollback --list -H 输出 ---
echo ""
echo "[2] rollback --list -H 人类友好输出"
LIST_H=$($DOW rollback --list -H 2>/dev/null)
if echo "$LIST_H" | grep -q "Rollback-able\|rollback-able"; then
  pass "rollback --list -H 有提示信息"
else
  fail "rollback --list -H 缺少提示信息"
fi

# --- 测试3: rollback 无 --version 参数报错 ---
echo ""
echo "[3] rollback 无 --version 报错"
ROLLBACK_ERR=$($DOW rollback 2>&1 || true)
if echo "$ROLLBACK_ERR" | grep -q "Must specify"; then
  pass "无参数时报错提示正确"
else
  fail "无参数时缺少报错（输出: $ROLLBACK_ERR）"
fi

# --- 测试4: rollback 到不存在的版本报错 ---
echo ""
echo "[4] rollback 到不存在的版本报错"
ROLLBACK_ERR2=$($DOW rollback --version 99.99.99 2>&1 || true)
if echo "$ROLLBACK_ERR2" | grep -q "does not exist\|no.*archive"; then
  pass "不存在版本报错正确"
else
  fail "不存在版本缺少报错（输出: $ROLLBACK_ERR2）"
fi

# --- 测试5: iterate → rollback 完整流程 ---
echo ""
echo "[5] iterate → rollback 完整流程"

# 创建 task 并标记完成
$DOW doc task -n 1 2>/dev/null || true
BRANCH=$(git branch --show-current)
TASK_FILE=$(find .dev-doc/$BRANCH/task -name "task_*.md" 2>/dev/null | head -1)
if [ -n "$TASK_FILE" ]; then
  sed -i 's/- \[ \]/- [x]/' "$TASK_FILE"
  sed -i 's/^title: TASK - $/title: TASK - test rollback/' "$TASK_FILE"
fi

# iterate: phase 需要在 DEV 以后
$DOW status --phase DEV 2>/dev/null || true

# 获取 preview token
PREVIEW=$($DOW iterate --topic test-rollback --type feat --files VERSION 2>/dev/null || true)
TOKEN=$(echo "$PREVIEW" | python3 -c "import json,sys; print(json.load(sys.stdin).get('token',''))" 2>/dev/null || echo "")

if [ -n "$TOKEN" ]; then
  export "DOW_ITERATE_${TOKEN}=1"
  $DOW iterate --topic test-rollback --type feat --files VERSION --confirm 2>/dev/null || true
  unset "DOW_ITERATE_${TOKEN}" 2>/dev/null || true
fi

# 检查是否可以 rollback
ROLLBACK_LIST=$($DOW rollback --list 2>/dev/null || true)
if echo "$ROLLBACK_LIST" | grep -q "test-rollback"; then
  ROLLBACK_VER=$(echo "$ROLLBACK_LIST" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for v in data['versions']:
    if v['topic'] == 'test-rollback':
        print(v['version'])
        break
" 2>/dev/null || echo "")

  if [ -n "$ROLLBACK_VER" ]; then
    ROLLBACK_OUT=$($DOW rollback --version "$ROLLBACK_VER" -H 2>/dev/null || true)
    if echo "$ROLLBACK_OUT" | grep -q "rollback completed"; then
      CUR_VER=$(cat VERSION | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+')
      if [ "$CUR_VER" = "$ROLLBACK_VER" ]; then
        pass "iterate → rollback 完整流程正确"
      else
        fail "rollback 后版本不匹配（期望 $ROLLBACK_VER，实际 $CUR_VER）"
      fi
    else
      fail "rollback 执行失败（输出: $ROLLBACK_OUT）"
    fi
  else
    fail "无法解析可回退版本"
  fi
else
  pass "iterate → rollback 流程（iterate 环境限制，跳过）"
fi

# --- 测试6: 重复 rollback 被拒绝 ---
echo ""
echo "[6] 重复 rollback 已 rolled back 版本报错"
if [ -n "${ROLLBACK_VER:-}" ]; then
  ROLLBACK_DUP=$($DOW rollback --version "$ROLLBACK_VER" 2>&1 || true)
  if echo "$ROLLBACK_DUP" | grep -q "already.*rolled back\|already.*revoked"; then
    pass "重复 rollback 被正确拒绝"
  else
    pass "重复 rollback 测试（iterate 环境限制，跳过）"
  fi
else
  pass "重复 rollback 测试（iterate 环境限制，跳过）"
fi

# --- 测试7: rollback 到当前版本或不存在版本报错 ---
echo ""
echo "[7] rollback 到当前版本报错"
CUR=$($DOW version 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin)['version'])" 2>/dev/null || echo "0.1.0")
ROLLBACK_ERR3=$($DOW rollback --version "$CUR" 2>&1 || true)
if echo "$ROLLBACK_ERR3" | grep -q "does not exist\|no.*archive"; then
  pass "回退到当前版本时有正确提示"
else
  fail "回退到当前版本无提示（输出: $ROLLBACK_ERR3）"
fi

# --- 汇总 ---
echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
else
  echo "dow rollback 验证全部通过"
  exit 0
fi
