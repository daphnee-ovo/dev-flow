#!/bin/bash
# T12 验证脚本：检查 hooks.json 配置和初始化脚本更新
# 验证 save-changelog.sh 替代 save-session.sh、validate.sh 和 scan-project.sh 的更新

PASS=0
FAIL=0
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

pass() { echo "  ✓ $1"; PASS=$((PASS + 1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL + 1)); }

echo "=== T12: hooks.json 配置和初始化脚本验证 ==="
echo ""

# --- 1. hooks.json 引用 save-changelog.sh ---
echo "[1] hooks.json 引用 save-changelog.sh"
if grep -q "save-changelog.sh" "$PROJECT_ROOT/hooks.json"; then
  pass "hooks.json 包含 save-changelog.sh"
else
  fail "hooks.json 未包含 save-changelog.sh"
fi

if grep -q "save-changelog.sh" "$PROJECT_ROOT/hooks/hooks.json"; then
  pass "hooks/hooks.json 包含 save-changelog.sh"
else
  fail "hooks/hooks.json 未包含 save-changelog.sh"
fi

# --- 2. hooks.json 不再引用 save-session.sh ---
echo ""
echo "[2] hooks.json 不包含 save-session.sh"
if grep -q "save-session" "$PROJECT_ROOT/hooks.json"; then
  fail "hooks.json 仍包含 save-session 引用"
else
  pass "hooks.json 无 save-session 引用"
fi

if grep -q "save-session" "$PROJECT_ROOT/hooks/hooks.json"; then
  fail "hooks/hooks.json 仍包含 save-session 引用"
else
  pass "hooks/hooks.json 无 save-session 引用"
fi

# --- 3. validate.sh 创建 task/ 目录 ---
echo ""
echo "[3] validate.sh 创建 task/ 目录"
if grep -q 'task' "$PROJECT_ROOT/scripts/init/validate.sh" | grep -q 'DOC_ROOT'; then
  : # 用下面的方式更准确
fi
if grep -q '\$DOC_ROOT/task' "$PROJECT_ROOT/scripts/init/validate.sh"; then
  pass "validate.sh 包含 \$DOC_ROOT/task 目录创建"
else
  fail "validate.sh 未包含 \$DOC_ROOT/task 目录创建"
fi

# --- 4. validate.sh 不再创建 session/memory 目录 ---
echo ""
echo "[4] validate.sh 不创建 session/memory 目录"
if grep -q "session" "$PROJECT_ROOT/scripts/init/validate.sh"; then
  fail "validate.sh 仍包含 session 引用"
else
  pass "validate.sh 无 session 引用"
fi

if grep -q "memory" "$PROJECT_ROOT/scripts/init/validate.sh"; then
  fail "validate.sh 仍包含 memory 引用"
else
  pass "validate.sh 无 memory 引用"
fi

# --- 5. validate.sh phase 枚举不含 MVP ---
echo ""
echo "[5] validate.sh phase 枚举不含 MVP"
PHASE_LINE=$(grep -E 'PRD|SPEC|TASK|DEV|TEST|DONE' "$PROJECT_ROOT/scripts/init/validate.sh" | grep -i "phase")
if echo "$PHASE_LINE" | grep -qi "MVP"; then
  fail "validate.sh phase 枚举包含 MVP"
else
  pass "validate.sh phase 枚举不含 MVP"
fi

# --- 6. validate.sh 有 CHANGELOG 检查逻辑 ---
echo ""
echo "[6] validate.sh 有 CHANGELOG 检查逻辑"
if grep -q "CHANGELOG" "$PROJECT_ROOT/scripts/init/validate.sh"; then
  pass "validate.sh 包含 CHANGELOG 检查逻辑"
else
  fail "validate.sh 未包含 CHANGELOG 检查逻辑"
fi

# --- 7. scan-project.sh 扫描 task/ 目录 ---
echo ""
echo "[7] scan-project.sh 扫描 task/ 目录"
if grep -q "task" "$PROJECT_ROOT/scripts/init/scan-project.sh"; then
  pass "scan-project.sh 包含 task 目录扫描"
else
  fail "scan-project.sh 未包含 task 目录扫描"
fi

# 更精确：检查是否输出 task 统计信息
if grep -q "task_summary" "$PROJECT_ROOT/scripts/init/scan-project.sh"; then
  pass "scan-project.sh 输出 task_summary 统计"
else
  fail "scan-project.sh 未输出 task_summary 统计"
fi

# --- 8. README hook 列表与现有聚合 hook 一致 ---
echo ""
echo "[8] README 不列出已删除 hook"
for doc in README.md README.zh-CN.md; do
  if grep -qE "check-task-completion|check-doc-sync|check-phase-completion|update-status" "$PROJECT_ROOT/$doc"; then
    fail "$doc 仍列出已删除 hook"
  else
    pass "$doc 未列出已删除 hook"
  fi
  if grep -q "post-write.sh" "$PROJECT_ROOT/$doc"; then
    pass "$doc 列出 post-write.sh"
  else
    fail "$doc 未列出 post-write.sh"
  fi
done

# --- 9. Claude plugin manifest 命令覆盖 ---
echo ""
echo "[9] Claude manifest 包含 /issue"
if python3 - "$PROJECT_ROOT/.claude-plugin/plugin.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
commands = set(data.get("commands", []))
sys.exit(0 if "./commands/issue.md" in commands else 1)
PY
then
  pass ".claude-plugin/plugin.json 包含 commands/issue.md"
else
  fail ".claude-plugin/plugin.json 缺少 commands/issue.md"
fi

# --- 10. Claude hook 使用插件根变量并引用路径 ---
echo ""
echo "[10] Claude hook 命令引用插件根"
if python3 - "$PROJECT_ROOT/hooks/hooks.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
for entries in data.get("hooks", {}).values():
    for entry in entries:
        for hook in entry.get("hooks", []):
            command = hook.get("command", "")
            if "scripts/hooks/" in command and not command.startswith('"${CLAUDE_PLUGIN_ROOT}"/'):
                sys.exit(1)
sys.exit(0)
PY
then
  pass "hooks/hooks.json 使用带引用的 CLAUDE_PLUGIN_ROOT"
else
  fail "hooks/hooks.json 未使用带引用的 CLAUDE_PLUGIN_ROOT"
fi

# --- 11. Codex manifest 不声明 unsupported hooks 字段 ---
echo ""
echo "[11] Codex manifest 不含 unsupported hooks 字段"
if python3 - "$PROJECT_ROOT/.codex-plugin/plugin.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
sys.exit(1 if "hooks" in data else 0)
PY
then
  pass ".codex-plugin/plugin.json 未声明 hooks 字段"
else
  fail ".codex-plugin/plugin.json 不应声明 hooks 字段"
fi

# === 总结 ===
echo ""
echo "=== 结果 ==="
echo "通过: $PASS  失败: $FAIL"
echo ""
if [ "$FAIL" -eq 0 ]; then
  echo "结论: 通过"
else
  echo "结论: 未通过"
fi
