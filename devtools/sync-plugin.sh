#!/bin/bash
# 同步当前项目到 Claude Code 插件缓存
PLUGIN_CACHE="$HOME/.claude/plugins/cache/daphnee-ovo/dev-flow/1.0.0"
rm -rf "$PLUGIN_CACHE"
cp -r "$(git rev-parse --show-toplevel)" "$PLUGIN_CACHE"
echo "[sync] 已同步到 $PLUGIN_CACHE"
