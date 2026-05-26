#!/bin/bash
# dow 编译部署脚本
# 编译 release 版本并部署到 scripts/bin/
# - 本地构建：直接覆盖 dow（原生二进制，零 wrapper 开销）
# - CI 交叉编译：指定 --dist 输出平台后缀二进制 + wrapper

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BIN_DIR="$PROJECT_ROOT/scripts/bin"

mkdir -p "$BIN_DIR"

# 检测当前平台
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}_${ARCH}" in
  linux_x86_64)    PLATFORM="linux-x86_64" ;;
  linux_aarch64)   PLATFORM="linux-aarch64" ;;
  darwin_arm64)    PLATFORM="darwin-arm64" ;;
  darwin_x86_64)   PLATFORM="darwin-x86_64" ;;
  *)               PLATFORM="unknown" ;;
esac

cd "$SCRIPT_DIR"

if [ "${1:-}" = "--dist" ]; then
  # 分发模式：输出平台后缀二进制 + wrapper
  cargo build --release
  rm -f "$BIN_DIR/dow-${PLATFORM}"
  cp "$SCRIPT_DIR/target/release/dow" "$BIN_DIR/dow-${PLATFORM}"
  strip "$BIN_DIR/dow-${PLATFORM}" 2>/dev/null || true
  chmod +x "$BIN_DIR/dow-${PLATFORM}"

  # 安装 wrapper 作为 dow 入口
  rm -f "$BIN_DIR/dow"
  cp "$BIN_DIR/dow-wrapper" "$BIN_DIR/dow"
  chmod +x "$BIN_DIR/dow"

  echo "[dow] 分发构建完成：scripts/bin/dow-${PLATFORM}"
  echo "[dow] wrapper 已安装：scripts/bin/dow"
else
  # 本地模式：直接覆盖 dow（无 wrapper 开销）
  cargo build --release
  rm -f "$BIN_DIR/dow"
  cp "$SCRIPT_DIR/target/release/dow" "$BIN_DIR/dow"
  chmod +x "$BIN_DIR/dow"

  echo "[dow] 编译完成：scripts/bin/dow (native ${PLATFORM})"
fi

set +e
cd "$PROJECT_ROOT" 2>/dev/null || true
"$BIN_DIR/dow" status --field phase >/dev/null 2>&1 && echo " → dow 工作正常" || echo " → dow 验证失败"
exit 0
