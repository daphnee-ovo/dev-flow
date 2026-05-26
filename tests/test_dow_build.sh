#!/bin/bash
# T002 验证：跨平台 wrapper + build.sh --dist
set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$PROJ_ROOT/scripts/bin"
PASS=0; FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

echo "=== T002: dow build ==="

# 1. 本地模式
echo ""
echo "[1] bash dow/build.sh（本地模式）"
bash "$PROJ_ROOT/dow/build.sh" >/dev/null 2>&1
if file "$BIN_DIR/dow" | grep -q "ELF"; then
  pass "本地模式生成 ELF 二进制"
else
  fail "本地模式未生成 ELF 二进制"
fi

# 2. 分发模式
echo ""
echo "[2] bash dow/build.sh --dist"
bash "$PROJ_ROOT/dow/build.sh" --dist >/dev/null 2>&1
PLATFORM="linux-x86_64"
if [ -f "$BIN_DIR/dow-${PLATFORM}" ]; then
  pass "生成 dow-${PLATFORM}"
else
  fail "未生成 dow-${PLATFORM}"
fi

if file "$BIN_DIR/dow" | grep -q "shell script\|text"; then
  pass "dow 入口为 wrapper 脚本"
else
  fail "dow 入口不是 wrapper 脚本"
fi

# 3. wrapper 模式功能正常
echo ""
echo "[3] wrapper 模式执行"
OUT=$("$BIN_DIR/dow" version -H 2>&1)
if echo "$OUT" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  pass "wrapper 模式 dow version -H 正常输出"
else
  fail "wrapper 模式输出异常: $OUT"
fi

# 4. wrapper 对不支持平台报错
echo ""
echo "[4] wrapper 不支持平台处理"
# 模拟：临时删除对应二进制，wrapper 应报错
mv "$BIN_DIR/dow-${PLATFORM}" "$BIN_DIR/dow-${PLATFORM}.bak"
if "$BIN_DIR/dow" version 2>/dev/null; then
  fail "缺少二进制时 wrapper 应返回 exit 1"
else
  pass "缺少二进制时 wrapper 返回 exit 1"
fi
mv "$BIN_DIR/dow-${PLATFORM}.bak" "$BIN_DIR/dow-${PLATFORM}"

# 5. 恢复本地模式
echo ""
echo "[5] 恢复本地模式"
bash "$PROJ_ROOT/dow/build.sh" >/dev/null 2>&1
rm -f "$BIN_DIR/dow-${PLATFORM}"
if file "$BIN_DIR/dow" | grep -q "ELF"; then
  pass "恢复为原生二进制"
else
  fail "恢复失败"
fi

echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="
[ "$FAIL" -eq 0 ] || exit 1
