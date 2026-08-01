#!/bin/bash
# Assemble each agent plugin into dist/<agent>/.
# Usage: bash devtools/assemble.sh <claude|codex|kiro|pi|all>
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist"

# Extract the version from VERSION.
VERSION_RAW="$(cat "$PROJECT_ROOT/VERSION" 2>/dev/null || echo "0.0.0")"
# Format: (branch)X.Y.Z → extract X.Y.Z.
VERSION="${VERSION_RAW##*)}"; VERSION="${VERSION:-$VERSION_RAW}"

install_command_skills() {
  local target_dir="$1"
  local managed_marker="$2"

  python3 - "$PROJECT_ROOT/plugin/commands" "$target_dir/skills" "$managed_marker" <<'PYEOF'
import json
import sys
from pathlib import Path

commands_dir = Path(sys.argv[1])
skills_dir = Path(sys.argv[2])
managed_marker = sys.argv[3] == "true"


def split_frontmatter(text):
    if not text.startswith("---\n"):
        return {}, text
    end = text.find("\n---", 4)
    if end == -1:
        return {}, text
    frontmatter = text[4:end]
    body_start = end + len("\n---")
    if text[body_start:body_start + 1] == "\n":
        body_start += 1
    fields = {}
    for line in frontmatter.splitlines():
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        fields[key.strip()] = value.strip()
    return fields, text[body_start:]


for command_file in sorted(commands_dir.glob("*.md")):
    command_name = command_file.stem
    text = command_file.read_text(encoding="utf-8")
    fields, body = split_frontmatter(text)
    description = fields.get("description") or f"Run the dev-flow /{command_name} workflow"
    skill_description = (
        f"{description}. Use this skill when the user requests the dev-flow "
        f"{command_name} workflow or expresses that intent."
    )

    skill_dir = skills_dir / command_name
    skill_dir.mkdir(parents=True, exist_ok=True)
    skill_file = skill_dir / "SKILL.md"
    skill_file.write_text(
        "---\n"
        f"name: {command_name}\n"
        f"description: {json.dumps(skill_description, ensure_ascii=False)}\n"
        "---\n\n"
        f"{body.lstrip()}",
        encoding="utf-8",
    )
    if managed_marker:
        (skill_dir / ".dev-flow-managed").touch()
PYEOF
}

install_codex_command_skills() {
  install_command_skills "$1" false
}

install_kiro_command_skills() {
  install_command_skills "$1" true
}

convert_agent_md_to_json() {
  local md_file="$1"
  local json_file="$2"
  local hooks_source="$PROJECT_ROOT/targets/kiro/agents/dev-flow/config.json"

  python3 - "$md_file" "$json_file" "$hooks_source" <<'PYEOF'
import json, sys

md_file, json_file, hooks_source = sys.argv[1], sys.argv[2], sys.argv[3]

with open(md_file, 'r') as f:
    content = f.read()

import os
agent_name = os.path.splitext(os.path.basename(md_file))[0]

lines = content.strip().split('\n')
description = lines[0].lstrip('# ').strip() if lines and lines[0].startswith('#') else f"{agent_name} agent"

hooks = {}
try:
    with open(hooks_source, 'r') as f:
        hooks = json.load(f).get('hooks', {})
except:
    pass

config = {
    "name": agent_name,
    "description": description,
    "instructions": content,
    "tools": ["read", "write", "shell", "web_search", "web_fetch", "multi_tool_use.parallel"],
    "hooks": hooks
}

with open(json_file, 'w') as f:
    json.dump(config, f, indent=2, ensure_ascii=False)
PYEOF
}

assemble_agent() {
  local agent="$1"
  local target_dir="$DIST_DIR/$agent"

  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  # Copy shared plugin content.
  # Kiro converts agents to JSON instead of copying Markdown; Pi uses skills.
  if [ "$agent" != "kiro" ] && [ "$agent" != "pi" ]; then
    cp -r "$PROJECT_ROOT/plugin/agents" "$target_dir/agents"
  fi

  # Copy the agent adapter.
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
  elif [ "$agent" = "kiro" ]; then
    install_kiro_command_skills "$target_dir"
    # Agents: flat JSON files (Kiro expects ~/.kiro/agents/<name>.json).
    mkdir -p "$target_dir/agents"
    # Main dev-flow agent, including hook definitions.
    cp "$PROJECT_ROOT/targets/kiro/agents/dev-flow/config.json" "$target_dir/agents/dev-flow.json"
    # Convert shared agents from Markdown to JSON and inject the same hooks.
    for agent_md in "$PROJECT_ROOT/plugin/agents"/*.md; do
      local aname="$(basename "$agent_md" .md)"
      convert_agent_md_to_json "$agent_md" "$target_dir/agents/$aname.json"
    done
  elif [ "$agent" = "pi" ]; then
    # Pi uses TypeScript extensions loaded from ~/.pi/agent/extensions/dev-flow/.
    # The extension entry point is index.ts.
    cp "$PROJECT_ROOT/targets/pi/extension.ts" "$target_dir/index.ts"
    # Skills use the same format as Codex: skills/<name>/SKILL.md.
    install_command_skills "$target_dir" false
  else
    echo "[assemble] Unknown agent: $agent" >&2
    exit 1
  fi

  echo "[assemble] ✓ $agent → dist/$agent/"
}

if [ -z "$1" ]; then
  echo "Usage: bash devtools/assemble.sh <claude|codex|kiro|pi|all>" >&2
  exit 1
fi

case "$1" in
  claude)
    assemble_agent claude
    ;;
  codex)
    assemble_agent codex
    ;;
  kiro)
    assemble_agent kiro
    ;;
  pi)
    assemble_agent pi
    ;;
  all)
    agents=()
    for target_dir in "$PROJECT_ROOT"/targets/*; do
      [ -d "$target_dir" ] || continue
      agents+=("${target_dir##*/}")
    done
    if [ "${#agents[@]}" -eq 0 ]; then
      echo "[assemble] No agent targets found under $PROJECT_ROOT/targets/" >&2
      exit 1
    fi
    for agent in "${agents[@]}"; do
      assemble_agent "$agent"
    done
    ;;
  *)
    echo "[assemble] Unknown argument: $1 (expected: claude, codex, kiro, pi, or all)" >&2
    exit 1
    ;;
esac
