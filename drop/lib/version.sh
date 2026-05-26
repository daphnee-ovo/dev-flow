#!/bin/bash
# 版本操作函数库
# 用法：source scripts/lib/version.sh

# 读取当前版本号
# 参数：$1 = 版本文件路径（默认 VERSION）
# 输出：版本字符串，失败返回空
version_read() {
  local version_file="${1:-VERSION}"
  if [ -f "$version_file" ]; then
    cat "$version_file" | tr -d '[:space:]'
  fi
}

# 校验版本号格式
# 参数：$1 = 版本字符串
# 返回：0=合法，1=非法
version_validate() {
  echo "$1" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'
}

# Bump 版本号
# 参数：$1 = 当前版本，$2 = bump 类型（major|minor|patch）
# 输出：新版本号字符串
version_bump() {
  local version="$1"
  local type="${2:-minor}"

  local major minor patch
  IFS='.' read -r major minor patch <<< "$version"

  case "$type" in
    major) major=$((major + 1)); minor=0; patch=0 ;;
    minor) minor=$((minor + 1)); patch=0 ;;
    patch) patch=$((patch + 1)) ;;
    *) echo "ERROR: unknown bump type: $type" >&2; return 1 ;;
  esac

  echo "${major}.${minor}.${patch}"
}

# 写入版本号到文件
# 参数：$1 = 新版本号，$2 = 版本文件路径（默认 VERSION）
version_write() {
  local new_version="$1"
  local version_file="${2:-VERSION}"

  if ! version_validate "$new_version"; then
    echo "ERROR: invalid version format: $new_version" >&2
    return 1
  fi

  echo "$new_version" > "$version_file"
}

# 检查 git tag 是否存在
# 参数：$1 = 版本号
# 返回：0=存在，1=不存在
version_tag_exists() {
  git tag -l "v$1" | grep -q "v$1"
}

# 创建 annotated git tag
# 参数：$1 = 版本号
# 返回：0=成功，1=失败
version_create_tag() {
  local version="$1"

  if version_tag_exists "$version"; then
    echo "ERROR: tag v$version already exists" >&2
    return 1
  fi

  git tag -a "v$version" -m "Release v$version"
}
