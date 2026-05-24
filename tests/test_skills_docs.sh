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

# --- 检查1: 三处 SKILL.md 的 description 字段含 /issue ---
echo "[1] 三处 SKILL.md description 字段含 /issue"

for f in \
  "$PROJ_ROOT/skills/dev-flow/SKILL.md" \
  "$PROJ_ROOT/.agents/skills/dev-flow/SKILL.md" \
  "$PROJ_ROOT/.claude/skills/dev-flow/SKILL.md"; do
  if [ ! -f "$f" ]; then
    fail "$f 文件不存在"
    continue
  fi
  # description 在前5行
  if head -5 "$f" | grep -q "/issue"; then
    pass "$f description 含 /issue"
  else
    fail "$f description 不含 /issue"
  fi
done

# --- 检查2: 三处 SKILL.md 命令映射表含 /issue ---
echo ""
echo "[2] 三处 SKILL.md 命令映射表含 /issue 行"

for f in \
  "$PROJ_ROOT/skills/dev-flow/SKILL.md" \
  "$PROJ_ROOT/.agents/skills/dev-flow/SKILL.md" \
  "$PROJ_ROOT/.claude/skills/dev-flow/SKILL.md"; do
  if [ ! -f "$f" ]; then
    fail "$f 文件不存在"
    continue
  fi
  # 命令映射表格式: | `/issue` | ... |
  if grep -q '`/issue`' "$f"; then
    pass "$f 命令表含 /issue"
  else
    fail "$f 命令表不含 /issue"
  fi
done

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

# --- 检查5: references/dev-flow-spec.md 目录结构含 task/ ---
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

# --- 检查7: references/dev-flow-spec.md archive 格式为 v<N>-<topic>/ ---
echo ""
echo "[7] references/dev-flow-spec.md archive 路径格式 v<N>-<topic>/"

if [ -f "$SPEC" ]; then
  if grep -qE 'v<N>-<topic>/' "$SPEC"; then
    pass "dev-flow-spec.md archive 格式为 v<N>-<topic>/"
  else
    fail "dev-flow-spec.md archive 格式不正确（期望 v<N>-<topic>/）"
  fi
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
