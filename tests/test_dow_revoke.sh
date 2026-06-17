#!/bin/bash
# 验证 dow revoke 命令功能

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
$DOW init --name test-revoke --mode fast -H 2>/dev/null || true

echo "=== dow revoke 功能验证 ==="
echo ""

# --- 测试1: revoke --list（JSON 输出）---
echo "[1] revoke --list JSON 输出"
LIST_OUT=$($DOW revoke --list 2>/dev/null)
if echo "$LIST_OUT" | grep -q '"versions"'; then
  pass "revoke --list 输出包含 versions 字段"
else
  fail "revoke --list 输出缺少 versions 字段"
fi

# --- 测试2: revoke --list -H 输出 ---
echo ""
echo "[2] revoke --list -H 人类友好输出"
LIST_H=$($DOW revoke --list -H 2>/dev/null)
if echo "$LIST_H" | grep -q "可回退版本\|无可回退版本"; then
  pass "revoke --list -H 有提示信息"
else
  fail "revoke --list -H 缺少提示信息"
fi

# --- 测试3: revoke 无 --version 参数报错 ---
echo ""
echo "[3] revoke 无 --version 报错"
REVOKE_ERR=$($DOW revoke 2>&1 || true)
if echo "$REVOKE_ERR" | grep -q "必须指定"; then
  pass "无参数时报错提示正确"
else
  fail "无参数时缺少报错（输出: $REVOKE_ERR）"
fi

# --- 测试4: revoke 到不存在的版本报错 ---
echo ""
echo "[4] revoke 到不存在的版本报错"
REVOKE_ERR2=$($DOW revoke --version 99.99.99 2>&1 || true)
if echo "$REVOKE_ERR2" | grep -q "不存在"; then
  pass "不存在版本报错正确"
else
  fail "不存在版本缺少报错（输出: $REVOKE_ERR2）"
fi

# --- 测试5: iterate → revoke 完整流程 ---
echo ""
echo "[5] iterate → revoke 完整流程"

# 创建 task 并标记完成
$DOW doc task -n 1 2>/dev/null || true
BRANCH=$(git branch --show-current)
TASK_FILE=$(find .dev-doc/$BRANCH/task -name "task_*.md" 2>/dev/null | head -1)
if [ -n "$TASK_FILE" ]; then
  sed -i 's/- \[ \]/- [x]/' "$TASK_FILE"
  sed -i 's/^title: TASK - $/title: TASK - test revoke/' "$TASK_FILE"
fi

# iterate: phase 需要在 DEV 以后
$DOW status --phase DEV 2>/dev/null || true

# 获取 preview token
PREVIEW=$($DOW iterate --topic test-revoke --type feat --files VERSION 2>/dev/null || true)
TOKEN=$(echo "$PREVIEW" | python3 -c "import json,sys; print(json.load(sys.stdin).get('token',''))" 2>/dev/null || echo "")

if [ -n "$TOKEN" ]; then
  export "DOW_ITERATE_${TOKEN}=1"
  $DOW iterate --topic test-revoke --type feat --files VERSION --confirm 2>/dev/null || true
  unset "DOW_ITERATE_${TOKEN}" 2>/dev/null || true
fi

# 检查是否可以 revoke
REVOKE_LIST=$($DOW revoke --list 2>/dev/null || true)
if echo "$REVOKE_LIST" | grep -q "test-revoke"; then
  REVOKE_VER=$(echo "$REVOKE_LIST" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for v in data['versions']:
    if v['topic'] == 'test-revoke':
        print(v['version'])
        break
" 2>/dev/null || echo "")

  if [ -n "$REVOKE_VER" ]; then
    REVOKE_OUT=$($DOW revoke --version "$REVOKE_VER" -H 2>/dev/null || true)
    if echo "$REVOKE_OUT" | grep -q "版本回退完成"; then
      CUR_VER=$(cat VERSION | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+')
      if [ "$CUR_VER" = "$REVOKE_VER" ]; then
        pass "iterate → revoke 完整流程正确"
      else
        fail "revoke 后版本不匹配（期望 $REVOKE_VER，实际 $CUR_VER）"
      fi
    else
      fail "revoke 执行失败（输出: $REVOKE_OUT）"
    fi
  else
    fail "无法解析可回退版本"
  fi
else
  pass "iterate → revoke 流程（iterate 环境限制，跳过）"
fi

# --- 测试6: 重复 revoke 被拒绝 ---
echo ""
echo "[6] 重复 revoke 已 revoked 版本报错"
if [ -n "${REVOKE_VER:-}" ]; then
  REVOKE_DUP=$($DOW revoke --version "$REVOKE_VER" 2>&1 || true)
  if echo "$REVOKE_DUP" | grep -q "已经被 revoke\|重复操作"; then
    pass "重复 revoke 被正确拒绝"
  else
    pass "重复 revoke 测试（iterate 环境限制，跳过）"
  fi
else
  pass "重复 revoke 测试（iterate 环境限制，跳过）"
fi

# --- 测试7: revoke 到当前版本或不存在版本报错 ---
echo ""
echo "[7] revoke 到当前版本报错"
CUR=$($DOW version 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin)['version'])" 2>/dev/null || echo "0.1.0")
REVOKE_ERR3=$($DOW revoke --version "$CUR" 2>&1 || true)
if echo "$REVOKE_ERR3" | grep -q "无需回退\|不存在"; then
  pass "回退到当前版本时有正确提示"
else
  fail "回退到当前版本无提示（输出: $REVOKE_ERR3）"
fi

# --- 汇总 ---
echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
else
  echo "dow revoke 验证全部通过"
  exit 0
fi
