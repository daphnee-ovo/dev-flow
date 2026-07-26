#!/bin/bash
# 本地开发部署：编译 + 组装 + 模拟安装 + dow setup
# 用法: bash devtools/deploy-local.sh <claude|codex|kiro|pi|all>
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ -z "$1" ]; then
  echo "用法: bash devtools/deploy-local.sh <claude|codex|kiro|pi|all>" >&2
  exit 1
fi

# 1. 编译 dow
echo "[deploy] 编译 dow..."
cd "$PROJECT_ROOT/dow"
cargo build --release
cd "$PROJECT_ROOT"

# 2. 组装 bundle
echo "[deploy] 组装插件..."
bash "$SCRIPT_DIR/assemble.sh" "$1"

# 3. 部署 dow 二进制（模拟 install.sh 下载后的放置）
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
rm -f "$BIN_DIR/dow"
cp "$PROJECT_ROOT/dow/target/release/dow" "$BIN_DIR/dow"
chmod +x "$BIN_DIR/dow"
echo "[deploy] ✓ dow → $BIN_DIR/dow"

# 4. 部署 bundle（模拟 install.sh 解压后的放置）
BUNDLE_DIR="$HOME/.local/share/dow/bundle"
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR"

case "$1" in
  all)
    cp -r "$PROJECT_ROOT/dist/claude" "$BUNDLE_DIR/claude"
    cp -r "$PROJECT_ROOT/dist/codex" "$BUNDLE_DIR/codex"
    cp -r "$PROJECT_ROOT/dist/kiro" "$BUNDLE_DIR/kiro"
    cp -r "$PROJECT_ROOT/dist/pi" "$BUNDLE_DIR/pi"
    ;;
  *)
    cp -r "$PROJECT_ROOT/dist/$1" "$BUNDLE_DIR/$1"
    ;;
esac
echo "[deploy] ✓ bundle → $BUNDLE_DIR"

# 5. 调用 dow setup 完成正式注册
echo "[deploy] 运行 dow setup..."
if ! "$BIN_DIR/dow" setup --agent "$1"; then
  echo "[deploy] ✗ dow setup failed; deployment did not complete" >&2
  exit 1
fi

# 6. Kiro: prompt user to set default agent (hooks require it)
if [ "$1" = "kiro" ] || [ "$1" = "all" ]; then
  echo ""
  echo "[deploy] ⚠ Kiro hooks require the dev-flow agent to be set as default."
  echo "         (kiro-default does not support hook configuration)"
  echo ""
  read -p "[deploy] Run 'kiro-cli agent set-default --name dev-flow' now? [Y/n] " answer
  case "${answer:-Y}" in
    [Yy]*)
      if command -v kiro-cli &>/dev/null; then
        kiro-cli agent set-default --name dev-flow
        echo "[deploy] ✓ dev-flow set as kiro default agent"
      else
        echo "[deploy] ✗ kiro-cli not found in PATH — please run manually:"
        echo "         kiro-cli agent set-default --name dev-flow"
      fi
      ;;
    *)
      echo "[deploy] Skipped. Run manually when ready:"
      echo "         kiro-cli agent set-default --name dev-flow"
      ;;
  esac
fi

echo "[deploy] 完成！"
