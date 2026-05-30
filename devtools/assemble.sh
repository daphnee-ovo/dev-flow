#!/bin/bash
# 组装各 agent 插件目录到 dist/
# 用法: bash devtools/assemble.sh <claude|codex|all>
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist"

# 从 VERSION 文件提取版本号
VERSION_RAW="$(cat "$PROJECT_ROOT/VERSION" 2>/dev/null || echo "0.0.0")"
# 格式: (branch)X.Y.Z → 提取 X.Y.Z
VERSION="${VERSION_RAW##*)}"; VERSION="${VERSION:-$VERSION_RAW}"

assemble_agent() {
  local agent="$1"
  local target_dir="$DIST_DIR/$agent"

  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  # 复制共享插件内容
  cp -r "$PROJECT_ROOT/plugin/skills" "$target_dir/skills"
  cp -r "$PROJECT_ROOT/plugin/commands" "$target_dir/commands"
  cp -r "$PROJECT_ROOT/plugin/agents" "$target_dir/agents"

  # 复制 agent 适配层
  if [ "$agent" = "claude" ]; then
    mkdir -p "$target_dir/.claude-plugin"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/claude/plugin.json" > "$target_dir/.claude-plugin/plugin.json"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/claude/marketplace.json" > "$target_dir/.claude-plugin/marketplace.json"
    mkdir -p "$target_dir/hooks"
    cp "$PROJECT_ROOT/targets/claude/hooks.json" "$target_dir/hooks/hooks.json"
  elif [ "$agent" = "codex" ]; then
    mkdir -p "$target_dir/.codex-plugin"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/codex/plugin.json" > "$target_dir/.codex-plugin/plugin.json"
    cp "$PROJECT_ROOT/targets/codex/app.json" "$target_dir/.app.json"
    mkdir -p "$target_dir/.agents/plugins"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/codex/personal-marketplace.json" > "$target_dir/.agents/plugins/marketplace.json"
    cp "$PROJECT_ROOT/targets/codex/hooks.json" "$target_dir/hooks.json"
    mkdir -p "$target_dir/hooks"
    cp "$PROJECT_ROOT/targets/codex/hooks.json" "$target_dir/hooks/hooks.json"
  else
    echo "[assemble] 未知 agent: $agent" >&2
    exit 1
  fi

  echo "[assemble] ✓ $agent → dist/$agent/"
}

if [ -z "$1" ]; then
  echo "用法: bash devtools/assemble.sh <claude|codex|all>" >&2
  exit 1
fi

case "$1" in
  claude)
    assemble_agent claude
    ;;
  codex)
    assemble_agent codex
    ;;
  all)
    assemble_agent claude
    assemble_agent codex
    ;;
  *)
    echo "[assemble] 未知参数: $1（可选: claude, codex, all）" >&2
    exit 1
    ;;
esac
