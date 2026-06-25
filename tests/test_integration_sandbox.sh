#!/usr/bin/env bash
# Integration test: Simulate workflow in isolated sandbox
# Tests SPEC-AC-001 through SPEC-AC-006 in realistic scenario
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$REPO_ROOT/tmp/test_target_project"

echo "=== Integration Test: Workflow Simulation ==="
echo "Test directory: $TEST_DIR"

# Clean and prepare test directory
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo ""
echo "--- Test Setup ---"

# Initialize a minimal project
echo "test-project" > README.md
git init -q
git config user.name "Test User"
git config user.email "test@example.com"
git add .
git commit -q -m "init: test project" --no-verify 2>/dev/null || git commit -q -m "init: test project"

echo "✓ Test project initialized"

# Initialize dev-flow
echo ""
echo "--- Testing dow init ---"
# Remove any existing .dev-doc entirely (clean slate)
rm -rf .dev-doc 2>/dev/null || true

if dow init --name "test-project" --mode "fast" 2>&1; then
  echo "✓ dow init succeeded"
else
  echo "❌ FAIL: dow init failed with exit code $?"
  exit 1
fi

# Verify .dev-doc structure (uses current git branch)
DOC_ROOT=$(dow status 2>&1 | grep -oP '(?<="doc_root": ")[^"]+')
if [[ -d "$DOC_ROOT" ]]; then
  echo "✓ .dev-doc structure created at $DOC_ROOT"
else
  echo "❌ FAIL: .dev-doc structure not created (expected $DOC_ROOT)"
  exit 1
fi

# Check STATUS.yaml
PHASE=$(dow status 2>&1 | grep -oP '(?<="phase": ")[^"]+')
if [[ "$PHASE" == "BRAINSTORM" ]]; then
  echo "✓ Initial phase is BRAINSTORM"
else
  echo "⚠ Warning: Initial phase is $PHASE (expected BRAINSTORM)"
fi

# Test project context generation (used in audit agents)
echo ""
echo "--- Testing project context generation ---"
CONTEXT_OUTPUT=$(dow hooks context)
if [[ -n "$CONTEXT_OUTPUT" ]]; then
  echo "✓ Project context generated"
  echo "Context length: $(echo "$CONTEXT_OUTPUT" | wc -c) bytes"
else
  echo "❌ FAIL: Project context empty"
  exit 1
fi

# Test document creation commands
echo ""
echo "--- Testing document creation ---"

# Clean any existing docs from previous runs
rm -f "$DOC_ROOT"/{BRAINSTORM,PRD,SPEC}.md 2>/dev/null || true

if dow brainstorm create 2>&1 && [[ -f "$DOC_ROOT/BRAINSTORM.md" ]]; then
  echo "✓ dow brainstorm create works, file created"
else
  echo "❌ FAIL: dow brainstorm create failed or file not created"
  exit 1
fi

if dow prd create 2>&1 && [[ -f "$DOC_ROOT/PRD.md" ]]; then
  echo "✓ dow prd create works, file created"
else
  echo "❌ FAIL: dow prd create failed or file not created"
  exit 1
fi

if dow spec create 2>&1 && [[ -f "$DOC_ROOT/SPEC.md" ]]; then
  echo "✓ dow spec create works, file created"
else
  echo "❌ FAIL: dow spec create failed or file not created"
  exit 1
fi

# Test schema commands (used by agents)
echo ""
echo "--- Testing schema commands ---"

SCHEMAS=("brainstorm" "prd" "spec" "task" "issue")
for schema in "${SCHEMAS[@]}"; do
  SCHEMA_OUTPUT=$(dow "$schema" schema 2>&1)
  if [[ -n "$SCHEMA_OUTPUT" ]] && echo "$SCHEMA_OUTPUT" | grep -qE "(\{|##|format|field|sections)"; then
    echo "✓ dow $schema schema returns valid format"
  else
    echo "❌ FAIL: dow $schema schema failed or empty"
    exit 1
  fi
done

# Test task creation (used in TASK phase)
echo ""
echo "--- Testing task creation ---"

TASK_JSON=$(cat <<'EOF'
{
  "title": "Integration test task",
  "done_when": [
    "file test.txt exists",
    "test.txt contains 'hello'"
  ],
  "priority": "P1",
  "files": {
    "create": ["test.txt"]
  }
}
EOF
)

if echo "$TASK_JSON" | dow task create 2>&1; then
  echo "✓ Task creation succeeded"
else
  echo "❌ FAIL: Task creation failed"
  exit 1
fi

# Verify task appears in list
if dow task list | grep -q "Integration test task"; then
  echo "✓ Task appears in list"
  TASK_ID=$(dow task list | grep -oE 'TASK-T[0-9]+' | tail -1)
  echo "  Task ID: $TASK_ID"

  # Verify task can be queried
  if dow task show "$TASK_ID" 2>&1 | grep -q "Integration test task"; then
    echo "✓ Task can be retrieved via show"
  else
    echo "⚠ Warning: Task show may have issues"
  fi
else
  echo "❌ FAIL: Task not in list"
  exit 1
fi

# Test issue creation (used in DEV/TEST phase)
echo ""
echo "--- Testing issue creation ---"

ISSUE_JSON=$(cat <<'EOF'
{
  "title": "Integration test issue",
  "description": "Test issue description",
  "severity": "P2",
  "source": "test"
}
EOF
)

if echo "$ISSUE_JSON" | dow issue create 2>&1; then
  echo "✓ Issue creation succeeded"
else
  echo "❌ FAIL: Issue creation failed"
  exit 1
fi

# Verify issue appears in list
if dow issue list | grep -q "Integration test issue"; then
  echo "✓ Issue appears in list"
  ISSUE_ID=$(dow issue list | grep -oE 'ISSUE-I[0-9]+' | tail -1)
  echo "  Issue ID: $ISSUE_ID"
else
  echo "❌ FAIL: Issue not in list"
  exit 1
fi

echo ""
echo "✅ Integration test PASSED"
echo ""
echo "Test project preserved at: $TEST_DIR"
echo "You can inspect it manually if needed."
