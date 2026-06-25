#!/bin/bash
# 运行所有 QA 测试

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export PATH="$SCRIPT_DIR/bin:$PATH"
TOTAL_PASS=0; TOTAL_FAIL=0
FAILED_SUITES=""

# 检查命名规范：tests/ 下的 .sh 文件必须为 test_*.sh
BAD_NAMES=""
for f in "$SCRIPT_DIR"/*.sh; do
  [ -f "$f" ] || continue
  fname="$(basename "$f")"
  case "$fname" in
    test_*.sh) ;;
    *) BAD_NAMES="$BAD_NAMES $fname" ;;
  esac
done
if [ -n "$BAD_NAMES" ]; then
  echo "ERROR: 以下文件不符合 test_*.sh 命名规范："
  for bad in $BAD_NAMES; do
    echo "  - $bad"
  done
  echo "请重命名后再运行。"
  exit 1
fi

for test_file in "$SCRIPT_DIR"/test_*.sh; do
  [ -f "$test_file" ] || continue
  [ "$(basename "$test_file")" = "test_all.sh" ] && continue
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
