#!/bin/bash
# 生成项目上下文摘要，作为 agent 公共输入
# 用法：bash context.sh [项目根目录]
# 输出：结构化文本到 stdout（不超过 200 行）

project_context() {
  local ROOT="${1:-.}"
  local MAX_LINES=200
  local OUTPUT=""

  # 检查目录是否存在
  if [ ! -d "$ROOT" ]; then
    echo "# 项目上下文"
    echo "（目录不存在：$ROOT）"
    return 0
  fi

  OUTPUT+="# 项目上下文\n"

  # === 空项目检测 ===
  local HAS_CONTENT=false
  if find "$ROOT" -maxdepth 1 -type f 2>/dev/null | head -1 | grep -q . || \
     find "$ROOT" -maxdepth 1 -mindepth 1 -type d ! -name '.git' 2>/dev/null | head -1 | grep -q .; then
    HAS_CONTENT=true
  fi
  if [ "$HAS_CONTENT" = "false" ]; then
    OUTPUT+="（空项目）\n"
    echo -e "$OUTPUT" | head -n "$MAX_LINES"
    return 0
  fi

  # === 技术栈推断 ===
  OUTPUT+="\n## 技术栈\n"
  local STACK=""
  [ -f "$ROOT/package.json" ] && STACK+="- Node.js/JavaScript\n"
  [ -f "$ROOT/tsconfig.json" ] && STACK+="- TypeScript\n"
  [ -f "$ROOT/requirements.txt" ] || [ -f "$ROOT/pyproject.toml" ] || [ -f "$ROOT/setup.py" ] && STACK+="- Python\n"
  [ -f "$ROOT/Cargo.toml" ] && STACK+="- Rust\n"
  [ -f "$ROOT/go.mod" ] && STACK+="- Go\n"
  [ -f "$ROOT/Gemfile" ] && STACK+="- Ruby\n"
  [ -f "$ROOT/pom.xml" ] || [ -f "$ROOT/build.gradle" ] && STACK+="- Java\n"
  if find "$ROOT" -maxdepth 2 -name "*.sh" -type f 2>/dev/null | head -1 | grep -q .; then
    STACK+="- Shell/Bash\n"
  fi
  [ -f "$ROOT/Dockerfile" ] && STACK+="- Docker\n"
  [ -f "$ROOT/docker-compose.yml" ] || [ -f "$ROOT/docker-compose.yaml" ] && STACK+="- Docker Compose\n"

  if [ -z "$STACK" ]; then
    OUTPUT+="- （无法自动推断）\n"
  else
    OUTPUT+="$STACK"
  fi

  # === 目录结构 ===
  OUTPUT+="\n## 目录结构\n"
  local TREE_OUT
  TREE_OUT=$(tree "$ROOT" -L 2 --dirsfirst -I 'node_modules|.git|__pycache__|.venv|venv|dist|build|.codegraph|tmp|temp' --noreport 2>/dev/null | head -60)
  if [ -n "$TREE_OUT" ]; then
    OUTPUT+="$TREE_OUT\n"
  else
    # fallback: find 格式化
    local FIND_OUT
    FIND_OUT=$(find "$ROOT" -maxdepth 2 -type d \
      ! -path '*/.git*' ! -path '*/node_modules*' ! -path '*/__pycache__*' \
      ! -path '*/.venv*' ! -path '*/venv*' ! -path '*/dist*' \
      ! -path '*/build*' ! -path '*/.codegraph*' ! -path '*/tmp*' ! -path '*/temp*' \
      2>/dev/null | sort | head -50 | sed "s|^$ROOT/||; s|^$ROOT$|.|")
    OUTPUT+="$FIND_OUT\n"
  fi

  # === 已有测试 ===
  if [ -d "$ROOT/tests" ] || [ -d "$ROOT/test" ]; then
    OUTPUT+="\n## 已有测试\n"
    local TEST_DIR="$ROOT/tests"
    [ ! -d "$TEST_DIR" ] && TEST_DIR="$ROOT/test"
    local TEST_FILES
    TEST_FILES=$(find "$TEST_DIR" -type f -name "test_*" -o -name "*_test.*" -o -name "*.test.*" 2>/dev/null | sort | head -20 | sed "s|^$ROOT/||")
    if [ -n "$TEST_FILES" ]; then
      OUTPUT+="$TEST_FILES\n"
    else
      OUTPUT+="（tests/ 目录存在但无匹配的测试文件）\n"
    fi
  fi

  # === 运行方式 ===
  OUTPUT+="\n## 运行方式\n"
  local RUN_INFO=""
  [ -f "$ROOT/Makefile" ] && RUN_INFO+="- Makefile 可用（make）\n"
  [ -f "$ROOT/package.json" ] && RUN_INFO+="- npm scripts 可用（npm run）\n"
  [ -f "$ROOT/Dockerfile" ] && RUN_INFO+="- Docker 构建可用\n"
  if find "$ROOT/scripts" -name "*.sh" -type f 2>/dev/null | head -1 | grep -q .; then
    RUN_INFO+="- bash scripts/ 目录下的脚本\n"
  fi
  if [ -z "$RUN_INFO" ]; then
    OUTPUT+="- （无标准运行入口）\n"
  else
    OUTPUT+="$RUN_INFO"
  fi

  # === 核心模块 ===
  OUTPUT+="\n## 核心模块\n"
  local MODULES=""
  for DIR in src lib scripts commands agents; do
    if [ -d "$ROOT/$DIR" ]; then
      local COUNT
      COUNT=$(find "$ROOT/$DIR" -type f 2>/dev/null | wc -l)
      MODULES+="- $DIR/（$COUNT 个文件）\n"
    fi
  done
  if [ -z "$MODULES" ]; then
    OUTPUT+="- （无标准模块目录）\n"
  else
    OUTPUT+="$MODULES"
  fi

  # 输出并限制行数
  echo -e "$OUTPUT" | head -n "$MAX_LINES"
}

# 如果直接执行（非 source），运行函数
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  project_context "$1"
fi
