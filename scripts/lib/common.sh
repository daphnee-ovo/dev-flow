#!/bin/bash
# Common helpers shared by dev-flow shell scripts.

devflow_repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

devflow_project_temp_dir() {
  local root
  root=$(devflow_repo_root)

  if [ -d "$root/temp" ] && [ ! -d "$root/tmp" ]; then
    echo "$root/temp"
  else
    echo "$root/tmp"
  fi
}

devflow_temp_file() {
  local prefix="${1:-devflow}"
  local temp_dir
  temp_dir=$(devflow_project_temp_dir)
  mkdir -p "$temp_dir"
  printf '%s/%s.%s.%s\n' "$temp_dir" "$prefix" "$$" "$(date +%s)"
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
  tmp=$(devflow_temp_file "$(basename "$file").yaml")
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

# 判断是否为 audit 模式（mode 以 "audit/" 开头）
# 参数：$1 = mode 字符串（可选，未传则从 STATUS.yaml 读取）
# 返回：0 = 是 audit 模式，1 = 不是
is_audit_mode() {
  local mode="$1"
  if [ -z "$mode" ]; then
    local root
    root=$(devflow_repo_root)
    local doc_root
    doc_root=$(devflow_resolve_doc_root "$root/dev-doc")
    mode=$(devflow_yaml_get "$doc_root/STATUS.yaml" "mode")
  fi
  case "$mode" in
    audit/*) return 0 ;;
    *) return 1 ;;
  esac
}

# 将 STATUS.yaml 切换为 audit 模式
# 参数：$1 = STATUS_FILE 路径
# 行为：读取当前 mode，写入 audit/<当前mode>，设置 phase 为 DEV，更新 updated 时间戳
# 如果已经是 audit 模式，不做任何操作直接返回 1
enter_audit_mode() {
  local status_file="$1"
  local current_mode
  current_mode=$(devflow_yaml_get "$status_file" "mode")

  # 已经是 audit 模式，不重复切换
  if is_audit_mode "$current_mode"; then
    return 1
  fi

  # 写入 audit 模式
  devflow_yaml_set "$status_file" "mode" "audit/$current_mode"
  devflow_yaml_set "$status_file" "phase" "DEV"
  devflow_yaml_set "$status_file" "updated" "$(date '+%Y-%m-%d %H:%M')"

  echo "[dev-flow] 检测到审计 issue，自动进入 audit 模式（原模式：$current_mode）"
}
