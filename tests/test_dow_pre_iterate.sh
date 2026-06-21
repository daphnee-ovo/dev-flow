#!/bin/bash
# 验证 dow iterate 的 preIterate steps

set -euo pipefail

PROJ_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOW="$PROJ_ROOT/dow/target/release/dow"
PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }
write_file() { printf "%s\n" "$2" > "$1"; }

TEST_DIR="$PROJ_ROOT/temp/test_pre_iterate_$$"
mkdir -p "$TEST_DIR"
trap 'rm -rf "$TEST_DIR"' EXIT

setup_project() {
  local name="$1"
  local dir="$TEST_DIR/$name"
  mkdir -p "$dir"
  cd "$dir"
  git init -q
  git config user.name "test"
  git config user.email "test@test.com"
  git commit --allow-empty -m "init: test project" -q
  "$DOW" init --name "$name" --mode fast -H >/dev/null 2>&1
  "$DOW" doc task -n 1 >/dev/null 2>&1
  local branch
  branch="$(git branch --show-current)"
  local task_file
  task_file="$(find ".dev-doc/$branch/task" -name "task_*.md" | head -1)"
  python3 - "$task_file" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
content = path.read_text()
content = content.replace("- [ ]", "- [x]")
content = content.replace("title: TASK - \n", "title: TASK - pre iterate\n")
path.write_text(content)
PY
  "$DOW" status --phase DEV >/dev/null 2>&1 || true
}

run_iterate_confirmed() {
  local topic="$1"
  local preview token
  preview="$("$DOW" iterate --topic "$topic" --type feat --files VERSION 2>/dev/null)"
  token="$(echo "$preview" | python3 -c "import json,sys; s=sys.stdin.read(); s=s[s.find('{'):]; print(json.loads(s).get('token',''))")"
  export "DOW_ITERATE_${token}=1"
  "$DOW" iterate --topic "$topic" --type feat --files VERSION --confirm >/dev/null 2>&1
  unset "DOW_ITERATE_${token}"
}

echo "=== dow iterate preIterate 验证 ==="
echo ""

echo "[1] sync-version 同步 Cargo/npm/pyproject 版本并进入 commit"
setup_project "sync-version"
write_file "Cargo.toml" $'# cargo manifest comment\n[package]\nname = "sync-version"\nversion = "9.9.9" # keep cargo inline comment\nedition = "2021"'
write_file "Cargo.lock" $'version = 3\n\n[[package]]\nname = "sync-version"\nversion = "9.9.9"'
write_file "package.json" $'{\n  "name": "sync-version",\n  "version": "9.9.9"\n}'
write_file "pyproject.toml" $'# pyproject comment\n[project]\nname = "sync-version"\nversion = "9.9.9" # keep pyproject inline comment'
write_file ".dev-doc/preIterate.ci" $'sync-version: Cargo.toml\nsync-version: package.json\nsync-version: pyproject.toml\nrun: python3 -c "from pathlib import Path; p=Path(\'Cargo.lock\'); p.write_text(p.read_text().replace(\'version = \\"9.9.9\\"\', \'version = \\"0.1.0\\"\'))"'
git add Cargo.toml Cargo.lock package.json pyproject.toml .dev-doc/preIterate.ci
git commit -m "test: add manifests" -q
run_iterate_confirmed "sync-version"
if grep -q 'version = "0.1.0"' Cargo.toml \
  && grep -q '# cargo manifest comment' Cargo.toml \
  && grep -q '# keep cargo inline comment' Cargo.toml \
  && grep -q 'version = "0.1.0"' Cargo.lock \
  && grep -q '"version": "0.1.0"' package.json \
  && grep -q 'version = "0.1.0"' pyproject.toml \
  && grep -q '# pyproject comment' pyproject.toml \
  && grep -q '# keep pyproject inline comment' pyproject.toml \
  && git show --name-only --format= HEAD | grep -q 'Cargo.toml' \
  && git show --name-only --format= HEAD | grep -q 'Cargo.lock' \
  && git show --name-only --format= HEAD | grep -q 'package.json' \
  && git show --name-only --format= HEAD | grep -q 'pyproject.toml'; then
  pass "sync-version 同步版本并纳入 iterate commit"
else
  fail "sync-version 未同步版本或未纳入 commit"
fi

echo ""
echo "[2] run step 在 git commit 前执行，产物进入 commit"
setup_project "run-step"
write_file ".dev-doc/preIterate.ci" "run: python3 -c \"from pathlib import Path; Path('generated.txt').write_text('marker')\""
git add .dev-doc/preIterate.ci
git commit -m "test: add preIterate run" -q
run_iterate_confirmed "run-step"
if [ "$(cat generated.txt 2>/dev/null)" = "marker" ] \
  && git show --name-only --format= HEAD | grep -q 'generated.txt'; then
  pass "run step 产物进入 iterate commit"
else
  fail "run step 未执行或产物未进入 commit"
fi

echo ""
echo "[3] run step 失败时阻断整个 iterate"
setup_project "failing-step"
write_file ".dev-doc/preIterate.ci" "run: exit 7"
git add .dev-doc/preIterate.ci
git commit -m "test: add failing preIterate" -q
before="$(git rev-parse HEAD)"
before_version="$(cat VERSION)"
preview="$("$DOW" iterate --topic failing-step --type feat --files VERSION 2>/dev/null)"
token="$(echo "$preview" | python3 -c "import json,sys; s=sys.stdin.read(); s=s[s.find('{'):]; print(json.loads(s).get('token',''))")"
export "DOW_ITERATE_${token}=1"
fail_out="$TEST_DIR/pre_iterate_fail.out"
if "$DOW" iterate --topic failing-step --type feat --files VERSION --confirm >"$fail_out" 2>&1; then
  fail "失败 step 未阻断 iterate"
else
  after="$(git rev-parse HEAD)"
  after_version="$(cat VERSION)"
  if [ "$before" = "$after" ] \
    && [ "$before_version" = "$after_version" ] \
    && grep -q 'preIterate step `run: exit 7` 失败' "$fail_out"; then
    pass "失败 step 阻断 commit 和版本变更"
  else
    fail "失败 step 的阻断结果不正确"
  fi
fi
unset "DOW_ITERATE_${token}"

echo ""
echo "[4] preIterate 失败回滚 sync-version 和 run 产物"
setup_project "rollback-step"
write_file "package.json" $'{\n  "name": "rollback-step",\n  "version": "9.9.9"\n}'
write_file ".dev-doc/preIterate.ci" "sync-version: package.json
run: python3 -c \"from pathlib import Path; Path('generated.txt').write_text('marker')\"
run: exit 7"
git add package.json .dev-doc/preIterate.ci
git commit -m "test: add rollback preIterate" -q
before="$(git rev-parse HEAD)"
before_version="$(cat VERSION)"
preview="$("$DOW" iterate --topic rollback-step --type feat --files VERSION 2>/dev/null)"
token="$(echo "$preview" | python3 -c "import json,sys; s=sys.stdin.read(); s=s[s.find('{'):]; print(json.loads(s).get('token',''))")"
export "DOW_ITERATE_${token}=1"
fail_out="$TEST_DIR/pre_iterate_rollback.out"
if "$DOW" iterate --topic rollback-step --type feat --files VERSION --confirm >"$fail_out" 2>&1; then
  fail "失败 step 未触发回滚阻断"
else
  after="$(git rev-parse HEAD)"
  after_version="$(cat VERSION)"
  if [ "$before" = "$after" ] \
    && [ "$before_version" = "$after_version" ] \
    && grep -q '"version": "9.9.9"' package.json \
    && [ ! -e generated.txt ] \
    && grep -q '已回滚 preIterate 修改' "$fail_out"; then
    pass "失败 step 回滚 sync-version 和 run 产物"
  else
    fail "失败 step 未正确回滚 preIterate 修改"
  fi
fi
unset "DOW_ITERATE_${token}"

echo ""
echo "[5] git add 失败时阻断 iterate commit"
setup_project "bad-files"
before="$(git rev-parse HEAD)"
before_version="$(cat VERSION)"
task_before="$(find .dev-doc/main/task -name 'task_*.md' -o -name 'done_task_*.md' | wc -l | tr -d ' ')"
preview="$("$DOW" iterate --topic bad-files --type feat --files missing-file.txt 2>/dev/null)"
token="$(echo "$preview" | python3 -c "import json,sys; s=sys.stdin.read(); s=s[s.find('{'):]; print(json.loads(s).get('token',''))")"
export "DOW_ITERATE_${token}=1"
fail_out="$TEST_DIR/git_add_fail.out"
if "$DOW" iterate --topic bad-files --type feat --files missing-file.txt --confirm >"$fail_out" 2>&1; then
  fail "git add 失败未阻断 iterate"
else
  after="$(git rev-parse HEAD)"
  after_version="$(cat VERSION)"
  task_after="$(find .dev-doc/main/task -name 'task_*.md' -o -name 'done_task_*.md' | wc -l | tr -d ' ')"
  if [ "$before" = "$after" ] \
    && [ "$before_version" = "$after_version" ] \
    && [ "$task_before" = "$task_after" ] \
    && grep -q 'iterate --files 路径不存在，已在归档前停止：missing-file.txt' "$fail_out"; then
    pass "git add 输入错误在归档前阻断 iterate"
  else
    fail "git add 失败的阻断结果不正确"
  fi
fi
unset "DOW_ITERATE_${token}"

echo ""
echo "=== 结果: $PASS 通过, $FAIL 失败 ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
else
  echo "dow iterate preIterate 验证全部通过"
  exit 0
fi
