#!/bin/bash
# Hook: Stop — 追加 CHANGELOG 记录
# 触发时机：会话结束时

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# 确定文档根目录
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.yaml" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

CHANGELOG="$DOC_ROOT/CHANGELOG.md"
DATE=$(date +%Y-%m-%d)
TIME=$(date +%H:%M)

# 推断 topic：从最近 git commit message 取，fallback 用 phase
TOPIC=$(git log --oneline -1 2>/dev/null | sed 's/^[a-f0-9]* //' | cut -c1-40)
if [ -z "$TOPIC" ]; then
  TOPIC=$(grep "^phase:" "$DOC_ROOT/STATUS.yaml" 2>/dev/null | sed 's/^phase: *//' | tr '[:upper:]' '[:lower:]')
fi
[ -z "$TOPIC" ] && TOPIC="session"

# 如果 CHANGELOG 不存在，创建头部
if [ ! -f "$CHANGELOG" ]; then
  printf "# Changelog\n\n" > "$CHANGELOG"
fi

# 清理 TOPIC 中的控制字符（保留中文等 UTF-8 多字节字符）
TOPIC=$(printf '%s' "$TOPIC" | sed 's/[[:cntrl:]]//g')

# 检查是否已有当天日期段
if ! grep -aq "^## $DATE" "$CHANGELOG"; then
  # 用 printf 追加而非 sed 插入（避免 header 大小写不匹配问题）
  printf "\n## %s\n" "$DATE" >> "$CHANGELOG"
fi

# 追加记录到文件末尾（当天最新在最后）
printf -- "- %s %s\n" "$TIME" "$TOPIC" >> "$CHANGELOG"

echo "[dev-flow] CHANGELOG 已更新：$TIME $TOPIC"
