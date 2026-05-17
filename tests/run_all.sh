#!/bin/bash
# 运行所有 QA 测试

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOTAL_PASS=0; TOTAL_FAIL=0
FAILED_SUITES=""

for test_file in "$SCRIPT_DIR"/test_*.sh; do
  [ -f "$test_file" ] || continue
  echo ""
  echo "================================================================"
  echo "  Running: $(basename "$test_file")"
  echo "================================================================"
  bash "$test_file"
  if [ $? -ne 0 ]; then
    FAILED_SUITES="$FAILED_SUITES $(basename "$test_file")"
  fi
done

echo ""
echo "================================================================"
echo "  ALL TESTS COMPLETE"
echo "================================================================"
if [ -n "$FAILED_SUITES" ]; then
  echo "FAILED SUITES:$FAILED_SUITES"
  exit 1
else
  echo "ALL SUITES PASSED"
  exit 0
fi
