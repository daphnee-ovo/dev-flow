#!/usr/bin/env bash
# Test SPEC-AC-001, SPEC-AC-002: Commands have audit flow and don't spawn artifact authors
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Test: Commands Have Audit Flow and Main Agent Execution ==="

FAILED=0

# SPEC-AC-001: Verify commands don't spawn artifact author subagents
echo ""
echo "--- Checking commands don't spawn artifact author subagents ---"

COMMANDS=(
  "plugin/commands/brainstorm.md"
  "plugin/commands/prd.md"
  "plugin/commands/spec.md"
)

for cmd in "${COMMANDS[@]}"; do
  echo "Checking $cmd..."

  # Should NOT contain "Spawn Agent" or "Agent Dispatch" patterns for artifact generation
  if grep -qiE "(spawn|agent dispatch).*(prd-agent|spec-agent|brainstorm-agent)" "$cmd" 2>/dev/null; then
    echo "❌ FAIL: $cmd still references spawning artifact author subagent"
    FAILED=1
  else
    echo "✓ $cmd does not spawn artifact author subagent"
  fi

  # Should contain "Main agent" or similar direct execution guidance
  if ! grep -qiE "(main agent|directly|you are acting)" "$cmd"; then
    echo "❌ FAIL: $cmd missing main agent execution guidance"
    FAILED=1
  else
    echo "✓ $cmd contains main agent execution guidance"
  fi
done

# SPEC-AC-002: Verify commands have audit steps in After Completion
echo ""
echo "--- Checking commands have audit steps ---"

for cmd in "${COMMANDS[@]}"; do
  echo "Checking $cmd for audit step..."

  if ! grep -qiE "(audit|spawn.*audit-agent)" "$cmd"; then
    echo "❌ FAIL: $cmd missing audit step"
    FAILED=1
  else
    echo "✓ $cmd has audit step"
  fi

  # Verify audit happens after user confirms direction
  if ! grep -qE "(after user confirm|user confirms direction)" "$cmd"; then
    echo "❌ FAIL: $cmd audit not tied to user confirmation"
    FAILED=1
  else
    echo "✓ $cmd audit triggered after user confirmation"
  fi
done

# Verify brainstorm.md specifically added audit (it didn't have it before)
echo ""
echo "--- Checking brainstorm.md added audit ---"
if grep -q "## Audit" plugin/commands/brainstorm.md; then
  echo "✓ brainstorm.md has Audit section"
else
  echo "❌ FAIL: brainstorm.md missing Audit section"
  FAILED=1
fi

# SPEC-AC-002: Verify audit agents receive decision summary and context
echo ""
echo "--- Checking audit input includes decision summary and context ---"
for cmd in "${COMMANDS[@]}"; do
  if grep -qE "(decision summary|dow hooks context)" "$cmd"; then
    echo "✓ $cmd audit receives decision summary and context"
  else
    echo "❌ FAIL: $cmd audit missing decision summary or context input"
    FAILED=1
  fi
done

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All command audit flow tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some command audit flow tests FAILED"
  exit 1
fi
