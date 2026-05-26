#!/bin/bash
# dow 编译部署脚本
# 编译 release 版本并复制到 scripts/bin/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$SCRIPT_DIR"
cargo build --release

mkdir -p "$PROJECT_ROOT/scripts/bin"
cp "$SCRIPT_DIR/target/release/dow" "$PROJECT_ROOT/scripts/bin/dow"
chmod +x "$PROJECT_ROOT/scripts/bin/dow"

echo "[dow] 编译完成：scripts/bin/dow"
"$PROJECT_ROOT/scripts/bin/dow" status --field phase 2>/dev/null && echo " → dow 工作正常" || echo " → dow 验证失败"
