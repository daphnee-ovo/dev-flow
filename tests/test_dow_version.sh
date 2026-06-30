#!/bin/bash
# T001 验证：dow version 子命令（在临时隔离目录中测试）
set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOW="$PROJ_ROOT/dow/target/release/dow"
PASS=0; FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

echo "=== T001: dow version ==="

# 创建临时隔离目录
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# 初始化一个 git repo 模拟真实项目
git -C "$TMPDIR" init -q
git -C "$TMPDIR" config user.name "test"
git -C "$TMPDIR" config user.email "test@test"
git -C "$TMPDIR" checkout -b main >/dev/null 2>&1
echo "(main)2.0.0" > "$TMPDIR/VERSION"
git -C "$TMPDIR" add -A
git -C "$TMPDIR" -c core.hooksPath=/dev/null commit -qm "init: test"

# 1. 读取版本
echo ""
echo "[1] dow version 读取"
OUT=$(cd "$TMPDIR" && $DOW version 2>&1) || true
if echo "$OUT" | grep -q '"version".*"2.0.0"'; then
  pass "JSON 输出包含正确 version"
else
  fail "JSON 输出异常: $OUT"
fi

# 2. --set 设定版本
echo ""
echo "[2] dow version --set"
OUT=$(cd "$TMPDIR" && $DOW version --set 4.0.0 2>&1)
if echo "$OUT" | grep -q '"action"' && echo "$OUT" | grep -q '"version".*"4.0.0"'; then
  pass "--set 4.0.0 成功"
else
  fail "--set 4.0.0 输出异常: $OUT"
fi

CONTENT=$(cat "$TMPDIR/VERSION")
if echo "$CONTENT" | grep -q "(main)4.0.0"; then
  pass "VERSION 文件已更新为 (main)4.0.0"
else
  fail "VERSION 文件内容异常: $CONTENT"
fi

# 3. --bump major
echo ""
echo "[3] dow version --bump major"
OUT=$(cd "$TMPDIR" && $DOW version --bump major 2>&1)
if echo "$OUT" | grep -q '"version".*"5.0.0"'; then
  pass "--bump major: 4.0.0 → 5.0.0"
else
  fail "--bump major 输出异常: $OUT"
fi

# 4. --set 非法格式
echo ""
echo "[4] dow version --set 非法格式"
if (cd "$TMPDIR" && $DOW version --set abc 2>/dev/null); then
  fail "--set abc 应返回 exit 1"
else
  pass "--set abc 返回 exit 1"
fi

# 5. -H 人类友好
echo ""
echo "[5] dow version -H"
echo "(main)3.0.0" > "$TMPDIR/VERSION"
OUT=$(cd "$TMPDIR" && $DOW version -H 2>&1)
if [ "$OUT" = "3.0.0" ]; then
  pass "-H 输出纯版本号"
else
  fail "-H 输出异常: $OUT"
fi

# 6. detached HEAD fallback
echo ""
echo "[6] dow version detached HEAD"
echo "(main)6.0.0" > "$TMPDIR/VERSION"
git -C "$TMPDIR" add -A && git -C "$TMPDIR" -c core.hooksPath=/dev/null commit -qm "feat: v6"
git -C "$TMPDIR" checkout --detach HEAD >/dev/null 2>&1
OUT=$(cd "$TMPDIR" && $DOW version 2>&1) && EXIT_CODE=0 || EXIT_CODE=$?
git -C "$TMPDIR" checkout main >/dev/null 2>&1

if [ "$EXIT_CODE" -eq 2 ]; then
  pass "detached HEAD 返回 exit code 2"
else
  fail "detached HEAD exit code 应为 2, 实际: $EXIT_CODE"
fi

if echo "$OUT" | grep -q '"warning"'; then
  pass "输出包含 warning 字段"
else
  fail "输出缺少 warning 字段: $OUT"
fi

if echo "$OUT" | grep -q '"branch".*"main"'; then
  pass "fallback 到 main 分支"
else
  fail "未 fallback 到 main: $OUT"
fi

# 7. detached HEAD write 拒绝
echo ""
echo "[7] dow version --set detached HEAD"
git -C "$TMPDIR" checkout --detach HEAD >/dev/null 2>&1
if (cd "$TMPDIR" && $DOW version --set 9.9.9 2>/dev/null); then
  fail "--set 在 detached HEAD 应返回 exit 1"
else
  pass "--set 在 detached HEAD 返回 exit 1"
fi
git -C "$TMPDIR" checkout main >/dev/null 2>&1

echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="
[ "$FAIL" -eq 0 ] || exit 1
