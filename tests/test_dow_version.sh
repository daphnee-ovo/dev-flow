#!/bin/bash
# T001 验证：dow version 子命令
set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOW="$PROJ_ROOT/dow/target/release/dow"
PASS=0; FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

echo "=== T001: dow version ==="

# 备份
cp "$PROJ_ROOT/VERSION" "$PROJ_ROOT/VERSION.bak"
trap 'cp "$PROJ_ROOT/VERSION.bak" "$PROJ_ROOT/VERSION"; rm -f "$PROJ_ROOT/VERSION.bak"' EXIT

# 1. 读取版本
echo ""
echo "[1] dow version 读取"
OUT=$($DOW version 2>&1)
if echo "$OUT" | grep -q '"version"'; then
  pass "JSON 输出包含 version 字段"
else
  fail "JSON 输出缺少 version 字段"
fi

# 2. --set 设定版本
echo ""
echo "[2] dow version --set"
OUT=$($DOW version --set 4.0.0 2>&1)
if echo "$OUT" | grep -q '"action"' && echo "$OUT" | grep -q '"version".*"4.0.0"'; then
  pass "--set 4.0.0 成功"
else
  fail "--set 4.0.0 输出异常: $OUT"
fi

# 多分支格式：(branch)version
CONTENT=$(cat "$PROJ_ROOT/VERSION")
BRANCH=$(git -C "$PROJ_ROOT" branch --show-current 2>/dev/null || echo "main")
if echo "$CONTENT" | grep -q "(${BRANCH})4.0.0"; then
  pass "VERSION 文件已更新为 (${BRANCH})4.0.0"
else
  fail "VERSION 文件内容异常: $CONTENT"
fi

# 3. --bump major
echo ""
echo "[3] dow version --bump major"
OUT=$($DOW version --bump major 2>&1)
if echo "$OUT" | grep -q '"version".*"5.0.0"'; then
  pass "--bump major: 4.0.0 → 5.0.0"
else
  fail "--bump major 输出异常: $OUT"
fi

# 4. --set 非法格式
echo ""
echo "[4] dow version --set 非法格式"
if $DOW version --set abc 2>/dev/null; then
  fail "--set abc 应返回 exit 1"
else
  pass "--set abc 返回 exit 1"
fi

# 5. -H 人类友好
echo ""
echo "[5] dow version -H"
BRANCH=$(git -C "$PROJ_ROOT" branch --show-current 2>/dev/null || echo "main")
echo "(${BRANCH})3.0.0" > "$PROJ_ROOT/VERSION"
OUT=$($DOW version -H 2>&1)
if [ "$OUT" = "3.0.0" ]; then
  pass "-H 输出纯版本号"
else
  fail "-H 输出异常: $OUT"
fi

echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="
[ "$FAIL" -eq 0 ] || exit 1
