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

# Test assemble for codex
echo ""
echo "--- Testing assemble for codex ---"
bash devtools/assemble.sh codex > /dev/null 2>&1

if [[ ! -d "dist/codex" ]]; then
  echo "❌ FAIL: dist/codex not created"
  FAILED=1
else
  echo "✓ dist/codex created"
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

# SPEC-AC-006: Verify test-agent unchanged
echo ""
echo "--- Verifying test-agent unchanged ---"
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

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All targets config and assemble tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some targets config and assemble tests FAILED"
  exit 1
fi
