#!/bin/bash
# T002 验证：cargo build --release
set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PASS=0; FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

echo "=== T002: dow build ==="

# 1. cargo build --release
echo ""
echo "[1] cargo build --release"
cd "$PROJ_ROOT/dow"
cargo build --release 2>/dev/null
BIN="$PROJ_ROOT/dow/target/release/dow"
FILE_OUT="$(file "$BIN" 2>/dev/null || true)"
if [ -x "$BIN" ] && echo "$FILE_OUT" | grep -Eq "ELF|Mach-O|PE32|executable"; then
  pass "release 构建生成可执行二进制"
else
  fail "release 构建未生成可执行二进制: $FILE_OUT"
fi

# 2. 二进制可执行且返回版本
echo ""
echo "[2] 二进制 --version"
OUT=$("$PROJ_ROOT/dow/target/release/dow" --version 2>&1)
if echo "$OUT" | grep -qE '^dow [0-9]+\.[0-9]+\.[0-9]+'; then
  pass "--version 输出正常: $OUT"
else
  fail "--version 输出异常: $OUT"
fi

echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="
[ "$FAIL" -eq 0 ] || exit 1
