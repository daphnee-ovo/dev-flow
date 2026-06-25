#!/usr/bin/env bash
# Test SPEC-AC-005: Task command has complexity routing logic
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Test: Task Command Complexity Routing ==="

FAILED=0
TASK_CMD="plugin/commands/task.md"

echo "Checking $TASK_CMD for complexity routing..."

# Check for complexity routing section
if ! grep -q "## Complexity Routing" "$TASK_CMD"; then
  echo "❌ FAIL: Missing '## Complexity Routing' section"
  FAILED=1
else
  echo "✓ Has '## Complexity Routing' section"
fi

# Check for low complexity description
if ! grep -qE "(Low complexity|default)" "$TASK_CMD"; then
  echo "❌ FAIL: Missing low complexity description"
  FAILED=1
else
  echo "✓ Has low complexity description"
fi

# Check for high complexity description
if ! grep -qE "High complexity.*(adversarial|subagent)" "$TASK_CMD"; then
  echo "❌ FAIL: Missing high complexity adversarial mode description"
  FAILED=1
else
  echo "✓ Has high complexity adversarial mode description"
fi

# Check for adversarial mode details
echo ""
echo "--- Checking adversarial mode details ---"

if ! grep -qE "(Agent A|task decomposer)" "$TASK_CMD"; then
  echo "❌ FAIL: Missing Agent A (decomposer) description"
  FAILED=1
else
  echo "✓ Has Agent A (decomposer) description"
fi

if ! grep -qE "(Agent B|task-challenger)" "$TASK_CMD"; then
  echo "❌ FAIL: Missing Agent B (challenger) description"
  FAILED=1
else
  echo "✓ Has Agent B (challenger) description"
fi

# Check for max 5 rounds convergence rule
if ! grep -qE "max 5 rounds" "$TASK_CMD"; then
  echo "❌ FAIL: Missing max 5 rounds convergence rule"
  FAILED=1
else
  echo "✓ Has max 5 rounds convergence rule"
fi

# Check for convergence criteria (empty output)
if ! grep -qE "(empty|no.*finding)" "$TASK_CMD"; then
  echo "❌ FAIL: Missing convergence criteria (empty findings)"
  FAILED=1
else
  echo "✓ Has convergence criteria"
fi

# Check for complexity signals
echo ""
echo "--- Checking complexity signals ---"
SIGNALS=(
  "multi-module"
  "cross-dependencies"
  "interfaces not"
  "shared state"
  "circular dependencies"
)

for signal in "${SIGNALS[@]}"; do
  if grep -qiE "$signal" "$TASK_CMD"; then
    echo "✓ Mentions '$signal' complexity signal"
  else
    echo "⚠ Warning: Missing '$signal' complexity signal (not critical)"
  fi
done

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All task complexity routing tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some task complexity routing tests FAILED"
  exit 1
fi
