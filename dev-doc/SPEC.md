# 技术规范（SPEC）

## 1. 概述

本次重构目标：解决 dev-doc 的三个结构性问题。

1. **session/ 目录定位模糊** — 替换为 CHANGELOG.md 追加式记录
2. **TASK 单文件膨胀** — 拆分为 `task/` 多文件，按批次管理
3. **/done 和 /iterate 在 mvp 模式下行为不一致** — 统一行为，去除 MVP phase

核心原则：最小侵入、保持 bash + markdown 技术栈不变、已有项目可迁移。

---

## 2. 变更范围

### 2.1 需要删除的文件

| 文件 | 原因 |
|------|------|
| `references/dev-doc/SESSION.md` | session 概念被 CHANGELOG 替代 |
| `scripts/hooks/save-changelog.sh` | 重写为 CHANGELOG 追加逻辑 |

### 2.2 需要新增的文件

| 文件 | 用途 |
|------|------|
| `references/dev-doc/CHANGELOG.md` | CHANGELOG 格式模板 |
| `references/dev-doc/TASK-FILE.md` | 新 task 单文件格式模板（替代原 TASK.md 模板） |
| `commands/issue.md` | /issue 命令定义 |

### 2.3 需要修改的文件

| 文件 | 改动概要 |
|------|----------|
| `scripts/hooks/inject-context.sh` | 从 task/ 多文件读取统计；issue/task 互斥展示；注入最近一条 CHANGELOG |
| `scripts/hooks/check-task-completion.sh` | 扫描 task/ 目录而非单个 TASK.md；done_ 前缀重命名逻辑 |
| `scripts/hooks/save-changelog.sh` | 重写：追加 CHANGELOG.md 而非创建 session 文件 |
| `scripts/hooks/check-phase-completion.sh` | task 文件路径变更适配 |
| `scripts/hooks/update-status.sh` | task/ 目录变更时也触发时间戳更新 |
| `scripts/init/validate.sh` | 校验 task/ 目录、去除 session/ 校验、加 CHANGELOG 检查 |
| `scripts/init/scan-project.sh` | 扫描 task/ 目录 |
| `commands/init.md` | 目录结构变更（task/ 替代 session/）；去除 MVP phase |
| `commands/task.md` | 输出路径改为 task/ 多文件 |
| `commands/done.md` | 按 mode 分级检查项 |
| `commands/iterate.md` | 归档 done_task_*、closed_issue_*；archive 命名改为 v<N>-<topic>；自动前置 /done |
| `commands/test.md` | TASK 引用改为 task/ 目录 |
| `commands/devtest.md` | TASK 引用改为 task/ 目录 |
| `commands/fix.md` | 无结构性变更，仅 DOC_ROOT 路径兼容 |
| `commands/status.md` | 从 task/ 多文件统计；去除 session 引用 |
| `commands/check.md` | 去除 session 检查；增加 CHANGELOG 检查 |
| `references/dev-doc/STATUS.yaml` | phase 枚举去除 MVP |
| `references/dev-doc/TASK.md` | 废弃（由 TASK-FILE.md 替代） |
| `references/dev-doc/ISSUE.md` | 更新为新格式（checkbox + 子字段） |
| `references/dev-flow-spec.md` | 更新目录结构、归档规则 |
| `hooks.json` / `hooks/hooks.json` | check-task-completion matcher 扩展到 task/ 路径 |
| `skills/dev-flow/SKILL.md` 等三处 | 更新命令列表（新增 /issue） |
| `CLAUDE.md` / `AGENTS.md` | 命令列表更新 |

---

## 3. 文件格式规范

### 3.1 Task 单文件格式（新）

路径：`dev-doc/task/task_<YYYY-MM-DD>_<seq>.md`

完成标记：hook 自动重命名为 `done_task_<YYYY-MM-DD>_<seq>.md`

```markdown
---
title: TASK - <批次主题>
nums: <任务总数>
---

- [ ] T1：<标题>
  - level: P0
  - details：<描述>
  - depends on：<依赖>
  - Done when：<完成标准>
- [ ] T2：<标题>
  - level: P1
  - details：<描述>
  - depends on：<依赖>
  - Done when：<完成标准>
```

字段说明：
- `title`：批次主题，描述这批任务的目标
- `nums`：该文件中的任务总数
- `level`：P0=阻塞 / P1=重要 / P2=可选
- `depends on`：前置依赖（可跨文件引用，格式 `<文件名>:T<N>`）
- `Done when`：可验证的完成标准

done_ 前缀触发条件：文件内所有 checkbox 均为 `[x]`。

### 3.2 Issue 文件格式（更新）

路径：`dev-doc/issue/issue_<source>_<YYYY-MM-DD>_<seq>.md`

关闭标记：hook 自动重命名为 `closed_issue_<source>_<YYYY-MM-DD>_<seq>.md`

```markdown
---
source: test | devtest | other
nums: <issue 总数>
---

- [ ] I1：<标题>
  - severity: P0
  - location：<文件路径:行号>
  - description：<具体描述>
  - reproduce：<复现方法，可选>
  - fix：<关闭时填写修复说明>
- [x] I2：<标题>
  - severity: P1
  - location：<文件路径:行号>
  - description：<描述>
  - fix：修改了缓存失效逻辑
```

变更点（对比现有格式）：
- 去掉 `modified_time`、`status`、`task` 字段 — 用 checkbox + 文件名前缀表达状态，更简洁
- 多个 issue 可以合并在同一文件中（按 source+date 分文件）
- checkbox 勾选 = 已关闭；文件内全部勾选 → hook 加 `closed_` 前缀

### 3.3 CHANGELOG.md 格式（新）

路径：`dev-doc/CHANGELOG.md`

```markdown
# Changelog

## 2026-05-16
- 14:30 fix-login-bug: 修复登录验证逻辑

## 2026-05-15
- 14:00 implement-auth: 完成认证模块
- 10:00 init-project: 项目初始化
```

追加规则：
- `save-session` hook（Stop 触发）追加一条记录
- 格式：`- HH:MM <topic>: <一句话摘要>`
- 如果当天日期段不存在，先插入 `## YYYY-MM-DD` 行
- topic 由 hook 从当前 phase + 最近 git commit message 推断

### 3.4 STATUS.yaml（更新）

```yaml
name: <项目名称>
phase: PRD | SPEC | TASK | DEV | TEST | DONE
mode: full | quick | fast | mvp
iteration: 1
updated: YYYY-MM-DD HH:MM
started: YYYY-MM-DD HH:MM
```

变更：`phase` 枚举中去除 `MVP`（mvp 是 mode 不是 phase）。

### 3.5 Archive 命名（更新）

旧：`archive/v<N>/`
新：`archive/v<N>-<topic>/`

topic 在 /iterate 时询问用户输入。

---

## 4. Hook 脚本设计

### 4.1 inject-context.sh — 重写

**触发**：UserPromptSubmit

**逻辑变更**：

```bash
# 1. 从 task/ 目录统计任务（替代从 TASK.md 读取）
TOTAL=0; DONE=0; P0=0; P1=0; P2=0
for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  TOTAL=$((TOTAL + $(grep -c "^- \[" "$f")))
  DONE=$((DONE + $(grep -c "^- \[x\]" "$f")))
  P0=$((P0 + $(grep -c "level: P0" "$f")))
  P1=$((P1 + $(grep -c "level: P1" "$f")))
  P2=$((P2 + $(grep -c "level: P2" "$f")))
done
# 已完成文件也计入
for f in "$DOC_ROOT/task/done_task_"*.md; do
  [ -f "$f" ] || continue
  COUNT=$(grep -c "^- \[" "$f")
  TOTAL=$((TOTAL + COUNT))
  DONE=$((DONE + COUNT))
done

# 2. 统计 issue
OPEN_ISSUES=0
if [ -d "$DOC_ROOT/issue" ]; then
  OPEN_ISSUES=$(find "$DOC_ROOT/issue" -name "issue_*.md" ! -name "closed_*" | wc -l)
fi

# 3. 基础状态输出
echo "[TASK] Total: $TOTAL | P0:$P0 P1:$P1 P2:$P2"
echo "[Issue] Total: $OPEN_ISSUES"

# 4. issue/task 互斥展示
if [ "$OPEN_ISSUES" -gt 0 ]; then
  echo "Current Issue："
  # 列出未关闭 issue 文件中的未勾选项标题
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    grep "^- \[ \]" "$f" | sed 's/^- \[ \] /  - /'
  done
else
  echo "Current Task："
  # 按优先级分层：P0 全 done → P1 → P2
  # 只列当前优先级未完成的标题
  # （具体实现见下方"优先级分层逻辑"）
fi

# 5. 注入最近一条 CHANGELOG（如果存在）
if [ -f "$DOC_ROOT/CHANGELOG.md" ]; then
  LAST_ENTRY=$(grep "^- " "$DOC_ROOT/CHANGELOG.md" | head -1)
  [ -n "$LAST_ENTRY" ] && echo "[Last] $LAST_ENTRY"
fi
```

**优先级分层逻辑**（task 展示部分）：

```bash
# 确定当前应展示的优先级
CURRENT_LEVEL=""
for LEVEL in P0 P1 P2; do
  HAS_UNDONE=0
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    # 检查该文件中是否有该优先级的未完成任务
    # 用 awk 在 checkbox 块内匹配 level
    if grep -B1 "level: $LEVEL" "$f" | grep -q "^- \[ \]"; then
      HAS_UNDONE=1
      break
    fi
  done
  if [ "$HAS_UNDONE" -eq 1 ]; then
    CURRENT_LEVEL="$LEVEL"
    break
  fi
done

# 列出该优先级的未完成 task 标题
if [ -n "$CURRENT_LEVEL" ]; then
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    # 提取未完成且属于当前优先级的任务标题
    awk '/^- \[ \]/{title=$0; getline; if(/level: '"$CURRENT_LEVEL"'/) print "  " title}' "$f" | sed 's/  - \[ \] /  - [ ] /'
  done
fi
```

**阶段规则注入**：保留现有 case 结构，但：
- 去除 `MVP)` 分支
- DEV 阶段提示中的 `TASK.md` 引用改为 `task/ 中的任务`
- 去除 session 相关规范提示

### 4.2 check-task-completion.sh — 重写

**触发**：PostToolUse(Edit) — matcher 保持 `Edit`

**逻辑变更**：

```bash
# 只在 DEV 阶段触发
PHASE=$(grep "^phase:" "$STATUS_FILE" | sed 's/^phase: *//')
[ "$PHASE" != "DEV" ] && exit 0

# === 检查 task/ 目录中的文件 ===
TOTAL=0; DONE=0
for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  TOTAL=$((TOTAL + $(grep -c "^- \[" "$f")))
  DONE=$((DONE + $(grep -c "^- \[x\]" "$f")))
done

[ "$TOTAL" -eq 0 ] && exit 0

UNDONE=$((TOTAL - DONE))

if [ "$UNDONE" -eq 0 ]; then
  echo "[dev-flow] 所有任务已完成（$DONE/$TOTAL）。"
  echo "→ 立即执行 /test 进行全量验证。"
else
  # 检查是否有刚完成的任务（检查被编辑的文件）
  CHANGED_FILE="${TOOL_INPUT_FILE_PATH:-$1}"
  if [[ "$CHANGED_FILE" == *task/task_*.md ]]; then
    TASK_NAME=$(grep "^- \[x\]" "$CHANGED_FILE" | tail -1 | sed 's/^- \[x\] //' | sed 's/（.*//;s/(.*$//')
    if [ -n "$TASK_NAME" ]; then
      echo "[dev-flow] 任务完成（$DONE/$TOTAL）：$TASK_NAME"
      echo "→ 自动触发 /devtest。"
    fi
  fi
fi

# === done_ 前缀自动重命名 ===
for f in "$DOC_ROOT/task/task_"*.md; do
  [ -f "$f" ] || continue
  FILE_TOTAL=$(grep -c "^- \[" "$f")
  FILE_DONE=$(grep -c "^- \[x\]" "$f")
  if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
    BASENAME=$(basename "$f")
    NEWNAME="$DOC_ROOT/task/done_$BASENAME"
    if [ ! -f "$NEWNAME" ]; then
      mv "$f" "$NEWNAME"
      echo "[dev-flow] 批次全部完成，已标记：done_$BASENAME"
    fi
  fi
done

# === issue 文件 closed_ 前缀重命名 ===
if [ -d "$DOC_ROOT/issue" ]; then
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    FILE_TOTAL=$(grep -c "^- \[" "$f")
    FILE_DONE=$(grep -c "^- \[x\]" "$f")
    if [ "$FILE_TOTAL" -gt 0 ] && [ "$FILE_TOTAL" -eq "$FILE_DONE" ]; then
      BASENAME=$(basename "$f")
      NEWNAME="$DOC_ROOT/issue/closed_$BASENAME"
      if [ ! -f "$NEWNAME" ]; then
        mv "$f" "$NEWNAME"
        echo "[dev-flow] Issue 全部关闭：closed_$BASENAME"
      fi
    fi
  done
fi
```

### 4.3 save-changelog.sh — 重写为 CHANGELOG 追加

**触发**：Stop

**新逻辑**：

```bash
#!/bin/bash
# Hook: Stop — 追加 CHANGELOG 记录

if [ ! -d "dev-doc" ]; then
  exit 0
fi

# DOC_ROOT 检测（保持现有分支逻辑）
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.yaml" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

CHANGELOG="$DOC_ROOT/CHANGELOG.md"
DATE=$(date +%Y-%m-%d)
TIME=$(date +%H:%M)

# 推断 topic：从最近 git commit message 取（fallback 用 phase）
TOPIC=$(git log --oneline -1 2>/dev/null | sed 's/^[a-f0-9]* //' | cut -c1-40)
if [ -z "$TOPIC" ]; then
  TOPIC=$(grep "^phase:" "$DOC_ROOT/STATUS.yaml" 2>/dev/null | sed 's/^phase: *//' | tr '[:upper:]' '[:lower:]')
fi
[ -z "$TOPIC" ] && TOPIC="session"

# 如果 CHANGELOG 不存在，创建头部
if [ ! -f "$CHANGELOG" ]; then
  echo "# Changelog" > "$CHANGELOG"
  echo "" >> "$CHANGELOG"
fi

# 检查是否已有当天日期段
if ! grep -q "^## $DATE" "$CHANGELOG"; then
  # 在文件头部（# Changelog 之后）插入新日期段
  sed -i "/^# Changelog$/a\\\\n## $DATE" "$CHANGELOG"
fi

# 在当天日期段下追加记录（插入到日期行之后的第一行）
sed -i "/^## $DATE$/a - $TIME $TOPIC" "$CHANGELOG"

echo "[dev-flow] CHANGELOG 已更新：$TIME $TOPIC"
```

### 4.4 check-phase-completion.sh — 适配

变更点：
- `TASK.md` 的检查路径改为 `task/*.md` 通配
- `Done when` 检查改为遍历 `task/task_*.md` 文件

### 4.5 update-status.sh — 适配

变更点：
- session 目录跳过逻辑保持，但改为不跳过 task/ 目录（task/ 下的变更应触发时间戳更新）
- 增加 CHANGELOG.md 跳过（避免 save-session 触发无限循环）

### 4.6 Hook 配置变更

`hooks.json` 和 `hooks/hooks.json` 中 `check-task-completion` 的 matcher：

当前：仅 `Edit`
变更：扩展为 `Write|Edit`（因为 task 文件可能是 Write 创建的）

---

## 5. 命令文件更新

### 5.1 commands/task.md

**输出路径变更**：

```
旧：<DOC_ROOT>/TASK.md
新：<DOC_ROOT>/task/task_<YYYY-MM-DD>_<seq>.md
```

agent prompt 中的输出路径说明需更新。task-agent 需要知道：
- 输出到 `task/` 目录
- 文件名格式 `task_<YYYY-MM-DD>_<seq>.md`
- 如果是追加批次（已有 task 文件），seq 递增

### 5.2 commands/done.md

**按 mode 分级检查**：

```bash
MODE=$(grep "^mode:" "$DOC_ROOT/STATUS.yaml" | sed 's/^mode: *//')

case "$MODE" in
  full)
    # PRD + SPEC + task 全完成 + TEST 全过 + 无 P0 issue
    [ ! -f "$DOC_ROOT/PRD.md" ] && BLOCKED="缺少 PRD.md"
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    ;;
  quick)
    # SPEC + task 全完成 + TEST 全过 + 无 P0 issue
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    ;;
  fast)
    # task 全完成 + TEST 全过 + 无 P0 issue
    ;;
  mvp)
    # SPEC 存在 + 代码可运行（用户确认）
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    # MVP 不强制 TEST.md，用户确认即可
    ;;
esac

# 通用检查（除 mvp 外）
if [ "$MODE" != "mvp" ]; then
  # task 全完成
  UNDONE=0
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    UNDONE=$((UNDONE + $(grep -c "^- \[ \]" "$f")))
  done
  [ "$UNDONE" -gt 0 ] && BLOCKED="$UNDONE 个任务未完成"
  
  # TEST.md 存在
  [ ! -f "$DOC_ROOT/TEST.md" ] && BLOCKED="未执行项目测试"
  
  # 无 P0 issue（不要求全部关闭，只要求无 P0）
  P0_OPEN=0
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    if grep -q "severity: P0" "$f" && grep -q "^- \[ \]" "$f"; then
      P0_OPEN=$((P0_OPEN + 1))
    fi
  done
  [ "$P0_OPEN" -gt 0 ] && BLOCKED="$P0_OPEN 个 P0 issue 未关闭"
fi
```

交付清单也需按 mode 调整（去除对 session 的检查）。

### 5.3 commands/iterate.md

**变更点**：

1. 前置检查增加：如果 STATUS 不是 DONE → 自动触发 /done（而非直接阻断）
2. 归档命名：`archive/v<N>-<topic>/`，iterate 时询问用户输入主题
3. 归档内容：
   - `done_task_*.md` → 移入 archive
   - `closed_issue_*.md` → 移入 archive
   - PRD.md / SPEC.md / TEST.md → 复制到 archive
   - CHANGELOG.md → 移入 archive（新迭代从空文件开始）
4. 保留在当前目录：未完成的 `task_*.md`、未关闭的 `issue_*.md`
5. 去除 session/ 归档逻辑
6. BRAINSTORM.md 不归档

### 5.4 commands/issue.md（新增）

```markdown
---
description: 手动创建 issue
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# ISSUE — 手动创建问题记录

## 执行步骤

1. 询问用户问题描述（标题、位置、描述、严重程度）
2. 确定 seq：
   ```bash
   DATE=$(date +%Y-%m-%d)
   NEXT_SEQ=$(find "$DOC_ROOT/issue" -name "issue_other_${DATE}_*.md" \
     -o -name "closed_issue_other_${DATE}_*.md" 2>/dev/null \
     | grep -oP "other_${DATE}_\K\d+" | sort -n | tail -1 || echo 0)
   NEXT_SEQ=$((NEXT_SEQ + 1))
   ```
3. 创建 issue 文件：`$DOC_ROOT/issue/issue_other_<DATE>_<seq>.md`
4. 写入新格式（checkbox + 子字段）
5. 提示用户：是否需要 `/fix` 修复

## 完成后

告知用户 issue 已创建，提供路径和下一步建议。
```

### 5.5 commands/status.md

- 从 `task/` 多文件统计任务进度
- 去除 session 最近记录，改为显示 CHANGELOG 最近条目
- 输出格式不变

### 5.6 commands/check.md

- 去除 session 记录检查（第 5 项）
- 增加 CHANGELOG 检查：如果有代码变更但 CHANGELOG 无今日记录 → 提醒
- TASK 引用改为 task/ 目录

### 5.7 commands/test.md / commands/devtest.md

- agent prompt 中的 `TASK.md` 引用改为 `task/ 目录下所有未完成的 task 文件`
- issue 输出格式保持新格式（checkbox + 子字段）

---

## 6. 初始化脚本更新

### 6.1 scripts/init/scan-project.sh

变更：
- `dev_doc` 部分扫描 `task/` 目录而非 `TASK.md`
- 不再扫描 `session/`

### 6.2 scripts/init/validate.sh

变更：

```bash
# 旧：创建 session/memory 目录
# 新：创建 task/ 目录
for dir in "$DOC_ROOT/issue" "$DOC_ROOT/task" "$DOC_ROOT/archive" "tests" "tmp"; do
  ...
done

# 去除 session 文件命名校验
# 新增 task 文件命名校验：
for f in "$DOC_ROOT/task/"*.md; do
  [ -f "$f" ] || continue
  BASENAME=$(basename "$f")
  if ! echo "$BASENAME" | grep -qE '^(done_)?task_[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]+\.md$'; then
    NEEDS_CONFIRM+=("task_bad_name:$BASENAME")
  fi
done

# STATUS.yaml phase 校验去除 MVP
if [ -n "$PHASE" ] && ! echo "$PHASE" | grep -qE '^(PRD|SPEC|TASK|DEV|TEST|DONE)$'; then
  WARNINGS+=("status_invalid_phase:$PHASE")
fi
```

### 6.3 commands/init.md

变更：
- 目录创建改为：`mkdir -p dev-doc/{issue,task,archive} tests tmp`
- 去除 `session/memory` 创建
- 初始阶段对照表中 mvp 模式初始阶段保持 SPEC（不再有 MVP phase）

---

## 7. 执行顺序与依赖

以下是实现时的依赖关系，必须按此顺序或并行执行：

```
阶段 1：基础格式定义（无依赖）
  ├── 新增 references/dev-doc/CHANGELOG.md
  ├── 新增 references/dev-doc/TASK-FILE.md
  ├── 更新 references/dev-doc/ISSUE.md
  ├── 更新 references/dev-doc/STATUS.yaml
  └── 删除 references/dev-doc/SESSION.md

阶段 2：Hook 脚本重写（依赖阶段 1 的格式定义）
  ├── 重写 scripts/hooks/inject-context.sh
  ├── 重写 scripts/hooks/check-task-completion.sh
  ├── 重写 scripts/hooks/save-changelog.sh
  ├── 适配 scripts/hooks/check-phase-completion.sh
  └── 适配 scripts/hooks/update-status.sh

阶段 3：命令文件更新（依赖阶段 1 + 2）
  ├── 更新 commands/task.md
  ├── 更新 commands/done.md
  ├── 更新 commands/iterate.md
  ├── 新增 commands/issue.md
  ├── 更新 commands/test.md
  ├── 更新 commands/devtest.md
  ├── 更新 commands/status.md
  ├── 更新 commands/check.md
  └── 更新 commands/init.md

阶段 4：配置与文档更新（依赖阶段 3）
  ├── 更新 hooks.json / hooks/hooks.json
  ├── 更新 references/dev-flow-spec.md
  ├── 更新 scripts/init/validate.sh
  ├── 更新 scripts/init/scan-project.sh
  ├── 更新 skills（三处）
  └── 更新 CLAUDE.md / AGENTS.md
```

阶段内各任务之间无依赖，可并行执行。

---

## 8. 向后兼容（迁移策略）

已有项目（使用旧 TASK.md + session/ 结构）需要迁移。迁移应在 `/init` 时自动检测并处理。

### 检测条件

```bash
# 如果存在旧 TASK.md 且不存在 task/ 目录 → 需要迁移
if [ -f "$DOC_ROOT/TASK.md" ] && [ ! -d "$DOC_ROOT/task" ]; then
  NEEDS_MIGRATION=true
fi
```

### 迁移步骤

1. **TASK.md → task/ 目录**：
   - 读取 TASK.md 内容
   - 创建 `task/` 目录
   - 将内容写入 `task/task_<today>_1.md`（保持格式不变，只是换了容器）
   - 保留原 TASK.md 为 `TASK.md.bak`（一轮迭代后可删除）

2. **session/ → CHANGELOG.md**：
   - 如果 `session/` 存在且有内容 → 从中提取摘要生成 CHANGELOG.md 初始内容
   - 如果 `session/` 为空 → 跳过
   - 保留原 `session/` 目录不删除（标记为 deprecated）

3. **STATUS.yaml phase=MVP → phase=DEV**：
   - 如果当前 phase 为 MVP → 改为 DEV

### 迁移在 validate.sh 中触发

迁移逻辑加入 validate.sh，作为 `auto_fixed` 类型输出。用户执行 `/init` 时自动完成。

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| task/ 多文件遍历性能 | 任务数量极大时 hook 超时 | hook 有 10s timeout；单项目不太可能超过 50 个 task 文件；必要时加 find 限深 |
| done_ 前缀重命名与 git 冲突 | 文件重命名导致 git diff 噪音 | 这是预期行为，归档时会清理；建议 .gitignore 不跟踪 done_task_* |
| 旧项目迁移中断 | TASK.md.bak 残留 | validate.sh 在下次 /init 时提醒清理 |
| inject-context 输出过长 | token 浪费 | 只展示当前优先级标题，不展示 details；issue 和 task 互斥 |
| CHANGELOG 追加并发写入 | 多个 agent 同时 Stop | bash 文件操作天然串行；hook 有 timeout 保护 |
| sed -i 跨平台差异 | macOS sed 需要 -i '' | 现有脚本已面对此问题，保持一致处理（项目主要在 Linux/WSL 使用） |
| issue checkbox 格式与 task 混淆 | hook 误判文件类型 | 通过路径区分（task/ vs issue/），不靠内容格式 |

---

## 10. 待定事项

1. **CHANGELOG topic 推断逻辑** — 当前设计用 git commit message，如果没有 commit（纯文档操作）如何推断？建议 fallback 为 phase 名称。已在设计中体现。

2. **done_task_ 文件是否进 git** — 建议跟踪（记录完成状态），但如果用户不希望 git 噪音可自行加 .gitignore。不强制。

3. **/issue 命令是否需要独立 agent** — 建议不需要，主 agent 直接执行即可（创建文件是简单操作）。

4. **task-agent.md 是否需要更新** — 需要。task-agent 的输出格式指引需从单文件改为多文件。这属于 commands/task.md 的联动变更。
