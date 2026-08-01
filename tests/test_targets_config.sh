#!/usr/bin/env bash
# Test SPEC-AC-006 & TASK-T006: Targets config updated, no old agent references
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Test: Targets Configuration and Assemble Output ==="

FAILED=0

# Test assemble for claude
echo ""
echo "--- Testing assemble for claude ---"
bash devtools/assemble.sh claude > /dev/null 2>&1

if [[ ! -d "dist/claude" ]]; then
  echo "❌ FAIL: dist/claude not created"
  FAILED=1
else
  echo "✓ dist/claude created"
fi

# Check assembled agents
echo "Checking assembled agents in dist/claude/agents/..."
NEW_AGENTS=(
  "brainstorm-audit-agent.md"
  "prd-audit-agent.md"
  "spec-audit-agent.md"
  "task-challenger-agent.md"
  "test-agent.md"
)

for agent in "${NEW_AGENTS[@]}"; do
  if [[ ! -f "dist/claude/agents/$agent" ]]; then
    echo "❌ FAIL: dist/claude/agents/$agent missing"
    FAILED=1
  else
    echo "✓ dist/claude/agents/$agent exists"
  fi
done

# Check old agents are NOT present
OLD_AGENTS=(
  "prd-agent.md"
  "spec-agent.md"
  "task-agent.md"
)

for agent in "${OLD_AGENTS[@]}"; do
  if [[ -f "dist/claude/agents/$agent" ]]; then
    echo "❌ FAIL: dist/claude/agents/$agent still exists (should be removed)"
    FAILED=1
  else
    echo "✓ dist/claude/agents/$agent correctly removed"
  fi
done

# Test assemble for every target directory
echo ""
echo "--- Testing target-driven assemble output ---"
bash devtools/assemble.sh all > /dev/null 2>&1

TARGET_AGENTS=()
for target_dir in targets/*; do
  [[ -d "$target_dir" ]] || continue
  TARGET_AGENTS+=("${target_dir##*/}")
done

if [[ ${#TARGET_AGENTS[@]} -eq 0 ]]; then
  echo "❌ FAIL: no agent targets found under targets/"
  FAILED=1
fi

for agent in "${TARGET_AGENTS[@]}"; do
  if [[ ! -d "dist/$agent" ]]; then
    echo "❌ FAIL: dist/$agent not created from targets/$agent"
    FAILED=1
  else
    echo "✓ dist/$agent created from targets/$agent"
  fi
done

echo ""
echo "--- Checking release packaging derives agents from targets/ ---"
if grep -q 'for target_dir in targets/\*' .github/workflows/release.yml && \
   grep -q 'cp -r "dist/${agent}" "_package/bundle/${agent}"' .github/workflows/release.yml && \
   grep -q 'Missing assembled bundle for target' .github/workflows/release.yml && \
   ! grep -q 'cp -r dist/claude\|cp -r dist/codex\|cp -r dist/kiro\|cp -r dist/pi' .github/workflows/release.yml; then
  echo "✓ release workflow packages targets dynamically"
else
  echo "❌ FAIL: release workflow still uses a hardcoded agent package list"
  FAILED=1
fi

# Check no references to old agents in any target config
echo ""
echo "--- Checking no references to old agents in target configs ---"
if grep -rE "(prd-agent|spec-agent|task-agent)" targets/ 2>/dev/null; then
  echo "❌ FAIL: Found references to old agents in targets/"
  FAILED=1
else
  echo "✓ No references to old agents in targets/"
fi

# Check no references to old agents in assembled output
if grep -rE "prd-agent|spec-agent|task-agent" dist/claude/ dist/codex/ 2>/dev/null; then
  echo "❌ FAIL: Found references to old agents in dist/"
  FAILED=1
else
  echo "✓ No references to old agents in dist/"
fi

# Verify TEST agent remains packaged
echo ""
echo "--- Verifying test-agent packaging ---"
if [[ -f "plugin/agents/test-agent.md" ]]; then
  echo "✓ plugin/agents/test-agent.md still exists"
else
  echo "❌ FAIL: plugin/agents/test-agent.md missing (should not be deleted)"
  FAILED=1
fi

if [[ -f "dist/claude/agents/test-agent.md" ]]; then
  echo "✓ dist/claude/agents/test-agent.md exists"
else
  echo "❌ FAIL: dist/claude/agents/test-agent.md missing"
  FAILED=1
fi

echo ""
echo "--- Verifying explicit TEST agent gate in assembled outputs ---"
TEST_SKILLS=(
  "dist/claude/commands/test.md"
  "dist/codex/skills/test/SKILL.md"
  "dist/kiro/skills/test/SKILL.md"
  "dist/pi/skills/test/SKILL.md"
)

for skill in "${TEST_SKILLS[@]}"; do
  if grep -q "explicitly requests TEST phase" "$skill"; then
    echo "✓ $skill requires explicit TEST entry"
  else
    echo "❌ FAIL: $skill is missing the explicit TEST entry gate"
    FAILED=1
  fi
done

echo ""
echo "--- Verifying user-only /fix invocation gate in assembled outputs ---"
FIX_SKILLS=(
  "dist/claude/commands/fix.md"
  "dist/codex/skills/fix/SKILL.md"
  "dist/kiro/skills/fix/SKILL.md"
  "dist/pi/skills/fix/SKILL.md"
)

for skill in "${FIX_SKILLS[@]}"; do
  if grep -q 'Only run this workflow when the user explicitly invokes `/fix`.' "$skill"; then
    echo "✓ $skill requires explicit /fix invocation"
  else
    echo "❌ FAIL: $skill is missing the explicit /fix invocation gate"
    FAILED=1
  fi
done

if grep -q '^disable-model-invocation: true$' "dist/claude/commands/fix.md"; then
  echo "✓ Claude /fix disables model invocation"
else
  echo "❌ FAIL: Claude /fix is missing disable-model-invocation"
  FAILED=1
fi

if grep -q '^user_only:' plugin/commands/fix.md; then
  echo "❌ FAIL: shared /fix command still declares unsupported user_only metadata"
  FAILED=1
else
  echo "✓ shared /fix command has no unsupported user_only metadata"
fi

for skill in "dist/codex/skills/fix/SKILL.md" "dist/kiro/skills/fix/SKILL.md" "dist/pi/skills/fix/SKILL.md"; do
  if ! grep -q '^user_only:' "$skill"; then
    echo "✓ $skill does not emit unsupported user_only metadata"
  else
    echo "❌ FAIL: $skill emits unsupported user_only metadata"
    FAILED=1
  fi
done

echo ""
echo "--- Verifying English skill metadata and cross-platform assembler parity ---"
DEFAULT_SKILLS=(
  "dist/codex/skills/status/SKILL.md"
  "dist/kiro/skills/status/SKILL.md"
  "dist/pi/skills/status/SKILL.md"
)

for skill in "${DEFAULT_SKILLS[@]}"; do
  if grep -q 'Use this skill when the user requests the dev-flow status workflow or expresses that intent.' "$skill" && \
     ! grep -q '当用户要求执行\|或表达对应流程意图' "$skill"; then
    echo "✓ $skill uses English skill metadata"
  else
    echo "❌ FAIL: $skill contains non-English or stale skill metadata"
    FAILED=1
  fi
done

if grep -q 'Use this skill when the user requests the dev-flow' devtools/assemble.ps1 && \
   grep -q '@("claude", "codex", "kiro", "pi")' devtools/assemble.ps1 && \
   ! grep -q 'user_only\|执行 dev-flow\|当用户要求执行\|未知 agent' devtools/assemble.ps1; then
  echo "✓ PowerShell assembler matches English and Pi assembly rules"
else
  echo "❌ FAIL: PowerShell assembler is out of sync"
  FAILED=1
fi

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All targets config and assemble tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some targets config and assemble tests FAILED"
  exit 1
fi
