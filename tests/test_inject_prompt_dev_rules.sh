#!/usr/bin/env bash
# Test SPEC-AC-004: Inject prompt contains DEV phase ad-hoc request rules
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Test: Inject Prompt DEV Ad-hoc Request Rules ==="

FAILED=0
INJECT_FILE="dow/references/inject_prompt/dev_flow.md"

echo "Checking $INJECT_FILE..."

# Check file exists
if [[ ! -f "$INJECT_FILE" ]]; then
  echo "❌ FAIL: $INJECT_FILE does not exist"
  exit 1
fi

# Check for ad-hoc request handling section
if ! grep -q "Handling ad-hoc requests during DEV" "$INJECT_FILE"; then
  echo "❌ FAIL: Missing 'Handling ad-hoc requests during DEV' section"
  FAILED=1
else
  echo "✓ Has 'Handling ad-hoc requests during DEV' section"
fi

# Check for complexity levels (S/M/L)
echo ""
echo "--- Checking complexity levels ---"
if ! grep -qE "complexity:.*S.*M.*L" "$INJECT_FILE"; then
  echo "❌ FAIL: Missing S/M/L complexity levels"
  FAILED=1
else
  echo "✓ Has S/M/L complexity levels"
fi

# Check for relation types
echo ""
echo "--- Checking relation types ---"
RELATIONS=("supplement" "disruptive" "independent")
for rel in "${RELATIONS[@]}"; do
  if grep -q "$rel" "$INJECT_FILE"; then
    echo "✓ Has '$rel' relation type"
  else
    echo "❌ FAIL: Missing '$rel' relation type"
    FAILED=1
  fi
done

# Check for decision rules
echo ""
echo "--- Checking decision rules ---"
RULES=(
  "S.*supplement"
  "M.*supplement"
  "L.*supplement"
  "independent.*new task"
  "disruptive.*pause"
)

for rule in "${RULES[@]}"; do
  if grep -qE "$rule" "$INJECT_FILE"; then
    echo "✓ Has rule matching '$rule'"
  else
    echo "❌ FAIL: Missing rule for '$rule'"
    FAILED=1
  fi
done

# Check for examples (few-shot)
echo ""
echo "--- Checking few-shot examples ---"
if ! grep -qE "Examples?:" "$INJECT_FILE"; then
  echo "❌ FAIL: Missing Examples section"
  FAILED=1
else
  echo "✓ Has Examples section"
fi

# Check specific example cases
EXAMPLES=(
  "Add a log line"
  "export CSV"
  "Redis"
)

for example in "${EXAMPLES[@]}"; do
  if grep -qi "$example" "$INJECT_FILE"; then
    echo "✓ Has example for '$example'"
  else
    echo "⚠ Warning: Missing example for '$example' (not critical)"
  fi
done

# Verify content is in English
echo ""
echo "--- Checking content language ---"
if grep -qE "(complexity|supplement|disruptive|independent)" "$INJECT_FILE"; then
  echo "✓ Content appears to be in English"
else
  echo "❌ FAIL: Content may not be in English"
  FAILED=1
fi

if [[ $FAILED -eq 0 ]]; then
  echo ""
  echo "✅ All inject prompt DEV rules tests PASSED"
  exit 0
else
  echo ""
  echo "❌ Some inject prompt DEV rules tests FAILED"
  exit 1
fi
