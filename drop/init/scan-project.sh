#!/bin/bash
# init 专用：扫描项目基本信息，输出结构化摘要
# 用法：bash scan-project.sh
# 替代 agent 手动探索，节省 token

echo "=== PROJECT SCAN ==="

# 项目名
NAME=""
if [ -f "package.json" ]; then
  NAME=$(awk -F\" '/"name"[[:space:]]*:/ { print $4; exit }' package.json)
elif [ -f "pyproject.toml" ]; then
  NAME=$(awk -F\" '/^name[[:space:]]*=/ { print $2; exit }' pyproject.toml)
elif [ -f "Cargo.toml" ]; then
  NAME=$(awk -F\" '/^name[[:space:]]*=/ { print $2; exit }' Cargo.toml)
fi
if [ -z "$NAME" ]; then
  NAME=$(basename "$(pwd)")
fi
echo "name: $NAME"

# 技术栈
STACK=""
[ -f "package.json" ] && STACK="$STACK node"
[ -f "tsconfig.json" ] && STACK="$STACK typescript"
[ -f "next.config.js" ] || [ -f "next.config.ts" ] || [ -f "next.config.mjs" ] && STACK="$STACK nextjs"
[ -f "pyproject.toml" ] || [ -f "setup.py" ] || [ -f "requirements.txt" ] && STACK="$STACK python"
[ -f "go.mod" ] && STACK="$STACK go"
[ -f "Cargo.toml" ] && STACK="$STACK rust"
[ -f "Gemfile" ] && STACK="$STACK ruby"
[ -f "pom.xml" ] || [ -f "build.gradle" ] && STACK="$STACK java"
echo "stack:$STACK"

# 构建/测试命令
echo ""
echo "commands:"
if [ -f "package.json" ]; then
  echo "  package_json_scripts:"
  awk '/"(build|test|dev|start|lint)"[[:space:]]*:/ { gsub(/^[[:space:]]+/, ""); gsub(/,$/, ""); print "    " $0 }' package.json
fi
if [ -f "Makefile" ]; then
  echo "  makefile_targets:"
  awk -F: '/^[A-Za-z_-]+:/ { print "    - " $1; count++; if (count >= 10) exit }' Makefile
fi
if [ -f "pyproject.toml" ] && grep -q "\[tool.pytest" pyproject.toml; then
  echo "  pytest: configured"
fi

# 代码风格配置
echo ""
echo "style:"
[ -f ".eslintrc" ] || [ -f ".eslintrc.js" ] || [ -f ".eslintrc.json" ] || [ -f "eslint.config.js" ] && echo "  - eslint"
[ -f ".prettierrc" ] || [ -f ".prettierrc.js" ] || [ -f "prettier.config.js" ] && echo "  - prettier"
[ -f "ruff.toml" ] || grep -q "\[tool.ruff\]" pyproject.toml 2>/dev/null && echo "  - ruff"
[ -f ".editorconfig" ] && echo "  - editorconfig"
[ -f "biome.json" ] && echo "  - biome"

# 目录结构
echo ""
echo "structure:"
find . -maxdepth 1 -type d ! -name '.' ! -name '.git' ! -name 'node_modules' ! -name '.next' ! -name '__pycache__' ! -name 'tmp' ! -name 'temp' | sort | sed 's|^\./|  - |'

# 代码规模
echo ""
FILE_COUNT=$(find . -type f ! -path './.git/*' ! -path './node_modules/*' ! -path './tmp/*' ! -path './temp/*' ! -path './.next/*' | wc -l)
echo "file_count: $FILE_COUNT"

# README 摘要
echo ""
if [ -f "README.md" ]; then
  echo "readme_first_line: $(head -5 README.md | grep -v '^$' | head -1)"
fi

# git 信息
echo ""
echo "git:"
BRANCH=$(git branch --show-current 2>/dev/null)
echo "  branch: $BRANCH"
COMMIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo 0)
echo "  commits: $COMMIT_COUNT"
echo "  recent:"
git log --oneline -5 2>/dev/null | sed 's/^/    /'

# 已有 dev-doc
echo ""
echo "dev_doc:"
if [ -d "dev-doc" ]; then
  find dev-doc \( -name "*.md" -o -name "*.yaml" \) 2>/dev/null | sort | sed 's/^/  - /'
  # task/ 目录统计
  if [ -d "dev-doc/task" ]; then
    ACTIVE_TASKS=$(find dev-doc/task -name "task_*.md" ! -name "done_*" 2>/dev/null | wc -l | tr -d '[:space:]')
    DONE_TASKS=$(find dev-doc/task -name "done_task_*.md" 2>/dev/null | wc -l | tr -d '[:space:]')
    echo "  task_summary: active=$ACTIVE_TASKS done=$DONE_TASKS"
  fi
  # issue/ 目录统计
  if [ -d "dev-doc/issue" ]; then
    OPEN_ISSUES=$(find dev-doc/issue -name "issue_*.md" ! -name "closed_*" 2>/dev/null | wc -l | tr -d '[:space:]')
    CLOSED_ISSUES=$(find dev-doc/issue -name "closed_issue_*.md" 2>/dev/null | wc -l | tr -d '[:space:]')
    echo "  issue_summary: open=$OPEN_ISSUES closed=$CLOSED_ISSUES"
  fi
else
  echo "  none"
fi

# 已有 agent 指令
echo ""
echo "agent_files:"
[ -f "CLAUDE.md" ] && echo "  - CLAUDE.md"
[ -f "AGENTS.md" ] && echo "  - AGENTS.md"
[ -f ".cursorrules" ] && echo "  - .cursorrules"
[ -f ".windsurfrules" ] && echo "  - .windsurfrules"

exit 0
