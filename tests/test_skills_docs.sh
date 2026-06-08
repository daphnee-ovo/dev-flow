#!/bin/bash
# T13 验证脚本：检查 skills 和项目级文档是否正确包含 /issue 命令及目录结构更新

set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

echo "=== T13: 验证 skills 和项目级文档 ==="
echo ""

# --- 检查1: plugin/skills/dev-flow/SKILL.md description 含 /issue ---
echo "[1] SKILL.md description 字段含 /issue"

SKILL="$PROJ_ROOT/plugin/skills/dev-flow/SKILL.md"
if [ ! -f "$SKILL" ]; then
  fail "$SKILL 文件不存在"
else
  if head -5 "$SKILL" | grep -q "/issue"; then
    pass "SKILL.md description 含 /issue"
  else
    fail "SKILL.md description 不含 /issue"
  fi
fi

# --- 检查2: SKILL.md 命令映射表含 /issue ---
echo ""
echo "[2] SKILL.md 命令映射表含 /issue 行"

if [ -f "$SKILL" ]; then
  if grep -q '`/issue`' "$SKILL"; then
    pass "SKILL.md 命令表含 /issue"
  else
    fail "SKILL.md 命令表不含 /issue"
  fi
fi

# --- 检查3: CLAUDE.md 命令表含 /issue ---
echo ""
echo "[3] CLAUDE.md 命令表含 /issue"

if grep -q '`/issue`\|/issue' "$PROJ_ROOT/CLAUDE.md"; then
  pass "CLAUDE.md 含 /issue"
else
  fail "CLAUDE.md 不含 /issue"
fi

# --- 检查4: AGENTS.md 命令表含 /issue ---
echo ""
echo "[4] AGENTS.md 命令表含 /issue"

if grep -q '`/issue`\|/issue' "$PROJ_ROOT/AGENTS.md"; then
  pass "AGENTS.md 含 /issue"
else
  fail "AGENTS.md 不含 /issue"
fi

# --- 检查5: references/dev-flow-spec.md 含 task/ 目录 ---
echo ""
echo "[5] references/dev-flow-spec.md 含 task/ 目录"

SPEC="$PROJ_ROOT/references/dev-flow-spec.md"
if [ ! -f "$SPEC" ]; then
  fail "$SPEC 文件不存在"
else
  if grep -q "task/" "$SPEC"; then
    pass "dev-flow-spec.md 含 task/ 目录"
  else
    fail "dev-flow-spec.md 不含 task/ 目录"
  fi
fi

# --- 检查6: references/dev-flow-spec.md 不含旧的 TASK.md 引用 ---
echo ""
echo "[6] references/dev-flow-spec.md 不含 TASK.md（已替换为 task/）"

if [ -f "$SPEC" ]; then
  if grep -q "TASK\.md" "$SPEC"; then
    fail "dev-flow-spec.md 仍含 TASK.md 引用（应已替换为 task/）"
  else
    pass "dev-flow-spec.md 不含 TASK.md"
  fi
fi

# --- 检查7: references/dev-flow-spec.md archive 使用 SQLite archive.db ---
echo ""
echo "[7] references/dev-flow-spec.md archive 使用 archive.db"

if [ -f "$SPEC" ]; then
  if grep -q ".dev-doc/archive.db" "$SPEC" && grep -q "dow archive list/show/tasks/issues/doc/stats" "$SPEC"; then
    pass "dev-flow-spec.md archive 使用 SQLite archive.db"
  else
    fail "dev-flow-spec.md archive 格式不正确（期望 .dev-doc/archive.db 和 dow archive 查询）"
  fi
fi

# --- 检查8: Claude plugin manifest 包含 /issue 命令 ---
echo ""
echo "[8] Claude plugin manifest 含 /issue"

if python3 - "$PROJ_ROOT/targets/claude/plugin.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
commands = set(data.get("commands", []))
sys.exit(0 if "./commands/issue.md" in commands else 1)
PY
then
  pass "targets/claude/plugin.json 含 /issue"
else
  fail "targets/claude/plugin.json 不含 /issue"
fi

# --- 汇总 ---
echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
else
  echo "T13 验证全部通过"
  exit 0
fi
