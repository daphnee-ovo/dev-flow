#!/bin/bash
# Common helpers shared by dev-flow shell scripts.

devflow_repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

devflow_resolve_doc_root() {
  local base="${1:-dev-doc}"
  local branch=""

  branch=$(git branch --show-current 2>/dev/null)
  if [ -n "$branch" ] && [ -f "$base/$branch/STATUS.yaml" ]; then
    echo "$base/$branch"
    return 0
  fi

  if [ -f "$base/STATUS.yaml" ]; then
    echo "$base"
    return 0
  fi

  if [ -d "$base" ]; then
    local first
    first=$(find "$base" -mindepth 2 -maxdepth 2 -name STATUS.yaml 2>/dev/null | sort | head -1)
    if [ -n "$first" ]; then
      dirname "$first"
      return 0
    fi
  fi

  echo "$base"
}

devflow_yaml_get() {
  local file="$1"
  local key="$2"
  [ -f "$file" ] || return 0
  awk -v key="$key" '
    $0 ~ "^" key ":" {
      value = $0
      sub("^" key ":[[:space:]]*", "", value)
      sub(/[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$file"
}

devflow_yaml_set() {
  local file="$1"
  local key="$2"
  local value="$3"
  local tmp

  [ -f "$file" ] || return 1
  tmp="${file}.tmp.$$"
  awk -v key="$key" -v value="$value" '
    BEGIN { done = 0 }
    $0 ~ "^" key ":" {
      print key ": " value
      done = 1
      next
    }
    { print }
    END {
      if (!done) print key ": " value
    }
  ' "$file" > "$tmp" && mv "$tmp" "$file"
}

devflow_json_field() {
  local field="$1"
  awk -v field="$field" '
    {
      pattern = "\"" field "\"[[:space:]]*:[[:space:]]*\""
      start = match($0, pattern)
      if (start) {
        rest = substr($0, RSTART + RLENGTH)
        end = index(rest, "\"")
        if (end > 0) {
          print substr(rest, 1, end - 1)
          exit
        }
      }
    }
  '
}

devflow_count_tasks() {
  local doc_root="${1:-dev-doc}"
  local total=0 done=0 cnt=0
  for f in "$doc_root/task/task_"*.md "$doc_root/task/done_task_"*.md; do
    [ -f "$f" ] || continue
    cnt=$(grep -c '^- \[' "$f" 2>/dev/null) || cnt=0
    total=$((total + cnt))
    cnt=$(grep -c '^- \[x\]' "$f" 2>/dev/null) || cnt=0
    done=$((done + cnt))
  done
  echo "$total $done"
}

devflow_count_open_issues() {
  local doc_root="${1:-dev-doc}"
  local open=0 cnt=0
  if [ -d "$doc_root/issue" ]; then
    for f in "$doc_root/issue/issue_"*.md; do
      [ -f "$f" ] || continue
      cnt=$(grep -c '^- \[ \]' "$f" 2>/dev/null) || cnt=0
      open=$((open + cnt))
    done
  fi
  echo "$open"
}
