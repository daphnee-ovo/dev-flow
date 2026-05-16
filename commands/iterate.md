---
description: 将已交付项目重新激活，进入下一轮迭代
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# ITERATE — 启动新迭代

## 前置检查（阻断）

1. 确认 `dev-doc/` 存在
2. 确认 STATUS 为 DONE（只有交付后才能迭代）
3. 如果 STATUS 不是 DONE → **停止，告知用户当前轮次尚未完成**

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.md" -path "*/*/STATUS.md" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

## 执行步骤

### 1. 确定当前版本号

```bash
# 从 STATUS.md 读取当前迭代版本，默认 v1
CURRENT_VERSION=$(grep "当前迭代" "$DOC_ROOT/STATUS.md" | grep -oP 'v\d+' || echo "v1")
NEXT_VERSION="v$((${CURRENT_VERSION#v} + 1))"
```

### 2. 归档当前文档

```bash
ARCHIVE_DIR="$DOC_ROOT/archive/$CURRENT_VERSION"
mkdir -p "$ARCHIVE_DIR/issue"

# 复制主文档
cp "$DOC_ROOT/PRD.md" "$ARCHIVE_DIR/" 2>/dev/null
cp "$DOC_ROOT/SPEC.md" "$ARCHIVE_DIR/" 2>/dev/null
cp "$DOC_ROOT/TASK.md" "$ARCHIVE_DIR/" 2>/dev/null
cp "$DOC_ROOT/TEST.md" "$ARCHIVE_DIR/" 2>/dev/null

# 归档已关闭 issue
mv "$DOC_ROOT/issue/"closed_issue_*.md "$ARCHIVE_DIR/issue/" 2>/dev/null
```

### 3. 询问迭代模式

向用户提问：

```
本次迭代的性质是？
1. 轻量迭代（bug 修复 / 小改动）— 保留现有 SPEC，直接进入开发
2. 标准迭代（新功能 / 大改动）— 从需求探索重新开始
```

### 4. 根据模式重置

**轻量迭代：**
- 保留 SPEC.md 不动
- 清空 TASK.md（只保留标题和格式）
- 删除 TEST.md
- STATUS 设为 DEV
- 提示用户：直接在 TASK.md 中添加任务，或用 `/task` 重新拆解

**标准迭代：**
- 清空 PRD.md / SPEC.md / TASK.md / TEST.md（只保留标题）
- STATUS 设为 PRD
- 提示用户：执行 `/prd` 开始需求探索

### 5. 更新 STATUS.md

更新以下字段：
- 当前迭代：`<NEXT_VERSION>`
- 当前阶段：DEV 或 PRD（取决于模式）
- 更新时间：当前时间
- 迭代启动时间：当前时间

### 6. 处理未关闭 issue

- 未关闭的 issue 保留在 `dev-doc/issue/` 中，**不归档**
- 告知用户有 N 个遗留 issue 带入了新迭代

## 输出格式

```
[dev-flow] 新迭代启动
━━━━━━━━━━━━━━━━━━━━━━
版本：<CURRENT_VERSION> → <NEXT_VERSION>
模式：轻量迭代 / 标准迭代
归档：已保存至 dev-doc/archive/<CURRENT_VERSION>/
遗留 Issue：N 个（已带入新迭代）

下一步：
  - <轻量：直接编写 TASK.md 或执行 /task>
  - <标准：执行 /prd 开始需求探索>
```

## 注意

- 归档是复制，不是移动（当前目录的文件会被清空/重置，但归档保留完整副本）
- session/ 目录不归档，持续积累
- 如果 archive 目录已存在同版本号，说明重复操作，停止并告知用户
