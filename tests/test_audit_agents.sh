#!/usr/bin/env bash
# Test SPEC-AC-001, SPEC-AC-002, SPEC-AC-003: Audit agent structure and content
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Test: Audit Agent Files Existence and Content ==="

# SPEC-AC-001 & SPEC-AC-002: Check audit agents exist
AGENTS=(
  "plugin/agents/brainstorm-audit-agent.md"
  "plugin/agents/prd-audit-agent.md"
  "plugin/agents/spec-audit-agent.md"
  "plugin/agents/task-challenger-agent.md"
)

FAILED=0

for agent in "${AGENTS[@]}"; do
  if [[ ! -f "$agent" ]]; then
    echo "❌ FAIL: $agent does not exist"
    FAILED=1
  else
    echo "✓ $agent exists"
  fi
done

# Check that old agents are deleted (TASK-T004)
OLD_AGENTS=(
  "plugin/agents/prd-agent.md"
  "plugin/agents/spec-agent.md"
  "plugin/agents/task-agent.md"
)

for agent in "${OLD_AGENTS[@]}"; do
  if [[ -f "$agent" ]]; then
    echo "❌ FAIL: Old agent $agent still exists (should be deleted)"
    FAILED=1
  else
    echo "✓ Old agent $agent correctly deleted"
  fi
done

# SPEC-AC-003: Verify audit agents have correct structure (Input/Output/Prohibited sections in English)
for agent in "${AGENTS[@]}"; do
  echo ""
  echo "--- Checking structure of $agent ---"

  if ! grep -q "## Input" "$agent"; then
    echo "❌ FAIL: $agent missing '## Input' section"
    FAILED=1
  else
    echo "✓ Has '## Input' section"
  fi

  if ! grep -q "## Output" "$agent"; then
    echo "❌ FAIL: $agent missing '## Output' section"
    FAILED=1
  else
    echo "✓ Has '## Output' section"
  fi

  if ! grep -q "## Prohibited" "$agent"; then
    echo "❌ FAIL: $agent missing '## Prohibited' section"
    FAILED=1
  else
    echo "✓ Has '## Prohibited' section"
  fi

  # Verify it's in English (check that prohibited section says "Do not see discussion process")
  if [[ "$agent" != *"task-challenger"* ]]; then
    if ! grep -q "Do not see discussion process" "$agent"; then
      echo "❌ FAIL: $agent not in English or missing key phrase in Prohibited"
      FAILED=1
    else
      echo "✓ Content in English with expected phrases"
    fi
  fi
done

# SPEC-AC-003: Verify audit agents explicitly state they don't receive discussion process
echo ""
echo "--- Verifying audit agents don't receive discussion context ---"
for agent in plugin/agents/*-audit-agent.md; do
  if grep -q "Do not see discussion process" "$agent"; then
    echo "✓ $agent explicitly states no discussion process"
  else
    echo "❌ FAIL: $agent missing 'Do not see discussion process' prohibition"
    FAILED=1
  fi
done

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All audit agent structure tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some audit agent structure tests FAILED"
  exit 1
fi
