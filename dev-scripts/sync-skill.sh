#!/bin/bash
# 同步 skills/dev-flow/SKILL.md 到所有副本位置
# 源：skills/dev-flow/SKILL.md
# 目标：.claude/skills/dev-flow/SKILL.md, .agents/skills/dev-flow/SKILL.md

set -e

ROOT="$(git rev-parse --show-toplevel)"
SOURCE="$ROOT/skills/dev-flow/SKILL.md"

TARGETS=(
    "$ROOT/.claude/skills/dev-flow/SKILL.md"
    "$ROOT/.agents/skills/dev-flow/SKILL.md"
)

if [ ! -f "$SOURCE" ]; then
    echo "[sync-skill] 源文件不存在：$SOURCE" >&2
    exit 1
fi

for target in "${TARGETS[@]}"; do
    mkdir -p "$(dirname "$target")"
    cp "$SOURCE" "$target"
    echo "[sync-skill] 已同步 → $target"
done

echo "[sync-skill] 完成（共 ${#TARGETS[@]} 个副本）"
