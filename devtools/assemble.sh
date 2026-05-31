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

extract_command_description() {
  local file="$1"
  awk '
    NR == 1 && $0 == "---" { in_fm = 1; next }
    in_fm && $0 == "---" { exit }
    in_fm && $0 ~ /^description:[[:space:]]*/ {
      sub(/^description:[[:space:]]*/, "")
      print
      exit
    }
  ' "$file"
}

append_command_body_without_frontmatter() {
  local file="$1"
  awk '
    NR == 1 && $0 == "---" { in_fm = 1; next }
    in_fm && $0 == "---" { in_fm = 0; next }
    !in_fm { print }
  ' "$file"
}

yaml_quote() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

install_codex_command_skills() {
  local target_dir="$1"
  local commands_dir="$PROJECT_ROOT/plugin/commands"
  local command_file command_name skill_dir description skill_description

  for command_file in "$commands_dir"/*.md; do
    command_name="$(basename "$command_file" .md)"
    skill_dir="$target_dir/skills/$command_name"
    description="$(extract_command_description "$command_file")"
    if [ -z "$description" ]; then
      description="执行 dev-flow /$command_name 流程"
    fi
    skill_description="$description。当用户要求执行 dev-flow $command_name 流程，或表达对应流程意图时使用。"

    mkdir -p "$skill_dir"
    {
      echo "---"
      echo "name: $command_name"
      echo "description: \"$(yaml_quote "$skill_description")\""
      echo "---"
      echo
      append_command_body_without_frontmatter "$command_file"
    } > "$skill_dir/SKILL.md"
  done
}

assemble_agent() {
  local agent="$1"
  local target_dir="$DIST_DIR/$agent"

  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  # 复制共享插件内容
  cp -r "$PROJECT_ROOT/plugin/skills" "$target_dir/skills"
  cp -r "$PROJECT_ROOT/plugin/agents" "$target_dir/agents"

  # 复制 agent 适配层
  if [ "$agent" = "claude" ]; then
    cp -r "$PROJECT_ROOT/plugin/commands" "$target_dir/commands"
    mkdir -p "$target_dir/.claude-plugin"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/claude/plugin.json" > "$target_dir/.claude-plugin/plugin.json"
    sed "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
      "$PROJECT_ROOT/targets/claude/marketplace.json" > "$target_dir/.claude-plugin/marketplace.json"
    mkdir -p "$target_dir/hooks"
    cp "$PROJECT_ROOT/targets/claude/hooks.json" "$target_dir/hooks/hooks.json"
  elif [ "$agent" = "codex" ]; then
    install_codex_command_skills "$target_dir"
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
