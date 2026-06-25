#!/usr/bin/env bash
# Master test runner for SPEC acceptance criteria
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT/tests"

echo "======================================"
echo "  Running All SPEC Acceptance Tests"
echo "======================================"
echo ""

FAILED_TESTS=()

# Run each test
TESTS=(
  "test_audit_agents.sh"
  "test_commands_audit_flow.sh"
  "test_task_complexity_routing.sh"
  "test_inject_prompt_dev_rules.sh"
  "test_targets_config.sh"
  "test_integration_sandbox.sh"
)

for test in "${TESTS[@]}"; do
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Running: $test"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if bash "$test"; then
    echo "✅ $test PASSED"
  else
    echo "❌ $test FAILED"
    FAILED_TESTS+=("$test")
  fi
done

echo ""
echo "======================================"
echo "  Test Summary"
echo "======================================"
echo "Total tests: ${#TESTS[@]}"
echo "Passed: $((${#TESTS[@]} - ${#FAILED_TESTS[@]}))"
echo "Failed: ${#FAILED_TESTS[@]}"

if [[ ${#FAILED_TESTS[@]} -eq 0 ]]; then
  echo ""
  echo "🎉 All tests PASSED!"
  exit 0
else
  echo ""
  echo "Failed tests:"
  for test in "${FAILED_TESTS[@]}"; do
    echo "  - $test"
  done
  exit 1
fi
