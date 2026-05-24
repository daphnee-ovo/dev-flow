# 技术规范（SPEC）

## 1. 概述

本次迭代目标：重构 agent 输入系统，让每个 agent 在不同模式下都能获得充分的工作上下文。

核心变更：
1. **引入项目上下文扫描** — 自动生成项目结构摘要，作为 agent 公共输入
2. **模式感知的输入模板** — 各 command 根据当前模式决定传入什么文档
3. **修复隔离边界** — 隔离"讨论过程"而非"项目现状"
4. **清理冗余/过时内容** — 删除已合并的独立 hook 文件、修正文档硬编码

---

## 2. 架构设计

### 核心理念变更

**之前**：agent 只看前置文档（PRD→SPEC→TASK），完全隔离项目代码
**之后**：agent 看前置文档 + 项目上下文，隔离的是讨论过程而非项目现状

```
Agent 输入 = 前置文档（按模式可选） + 项目上下文（始终提供） + agent prompt
```

### 项目上下文（Project Context）

新增 `scripts/lib/context.sh`，自动扫描项目生成结构化摘要：

```
项目上下文
├── 目录结构（tree -L 2，排除 node_modules/.git 等）
├── 技术栈推断（根据文件类型/配置文件判断）
├── 已有测试结构（tests/ 目录内容）
├── 运行方式（如果有 Makefile/package.json/Dockerfile）
└── 现有模块列表（scripts/*、src/* 等核心目录）
```

限制：输出不超过 200 行，避免输入 token 爆炸。

### 模式感知的输入映射

| Agent | full 模式输入 | quick 模式输入 | fast 模式输入 | mvp 模式输入 |
|-------|--------------|---------------|--------------|-------------|
| SPEC | PRD.md + 项目上下文 | 用户描述 + 项目上下文 | N/A | BRAINSTORM.md / 用户描述 + 项目上下文 |
| TASK | SPEC.md + 项目上下文 | SPEC.md + 项目上下文 | 用户描述 + 项目上下文 | N/A |
| TEST | SPEC.md + task files + 项目上下文 | 同 full | 同 full | N/A |
| devtest | Done when + SPEC 片段 + 项目上下文 | 同 full | 同 full | N/A |
| FIX | issue + SPEC 片段 + 项目上下文 | 同 full | 同 full | 同 full |

关键规则：
- 项目上下文始终传入（所有 agent、所有模式）
- 前置文档不存在时降级为"用户描述"（从 command prompt 中提取用户输入）
- command 前置检查从"文档不存在→阻断"改为"文档不存在→使用替代输入源"

### 模块划分

| 模块 | 职责 | 文件 |
|------|------|------|
| 上下文扫描 | 生成项目结构摘要 | `scripts/lib/context.sh` |
| command 调度 | 模式感知的 agent 输入组装 | `commands/*.md`（修改） |
| agent prompt | agent 角色定义（不变） | `agents/*-agent.md` |

### 目录结构变更

```
scripts/lib/
├── version.sh          # 已有
├── guard.sh            # 已有
└── context.sh          # 新增：项目上下文扫描

scripts/hooks/
├── inject-context.sh   # 保留
├── post-write.sh       # 保留
├── save-changelog.sh   # 保留
├── block-non-dev-edit.sh    # 保留
├── block-system-tmp.sh      # 保留
├── check-doc-sync.sh       # 删除（已合并到 post-write.sh）
├── check-phase-completion.sh # 删除（已合并）
├── check-task-completion.sh  # 删除（已合并）
└── update-status.sh         # 删除（已合并）
```

---

## 3. 技术选型

| 领域 | 选择 | 理由 |
|------|------|------|
| 上下文生成 | Bash 脚本（context.sh） | 与项目技术栈一致，无额外依赖 |
| 目录扫描 | `find` + `tree`（带 fallback） | tree 可能未安装，fallback 到 find 格式化输出 |
| 输出限制 | 硬编码 200 行上限 + `head` 截断 | 防止大项目 token 爆炸 |

---

## 4. 数据模型

### 4.1 context.sh 输出格式

```
# 项目上下文
## 技术栈
- Shell/Bash（主要）
- 无外部运行时依赖
## 目录结构
scripts/
├── commands/（6 个命令脚本）
├── hooks/（5 个 hook 脚本）
├── init/
└── lib/（3 个库文件）
tests/（13 个测试文件）
dev-doc/（流程文档）
## 已有测试
tests/test_commands.sh
tests/test_e2e_lifecycle.sh
...
## 运行方式
bash scripts/commands/<cmd>.sh
bash tests/test_<name>.sh
```

### 4.2 command 前置检查逻辑变更

```
# 之前
if [ ! -f PRD.md ]; then 停止; fi

# 之后
if [ -f PRD.md ]; then
  INPUT_DOC="PRD.md 内容"
else
  case $MODE in
    quick|mvp) INPUT_DOC="用户描述 + BRAINSTORM.md（如存在）" ;;
    *) 停止，告知先执行 /prd ;;
  esac
fi
```

---

## 5. 接口设计

### 5.1 scripts/lib/context.sh

```bash
#!/bin/bash
# 生成项目上下文摘要
# 用法：bash context.sh [项目根目录]
# 输出：结构化文本到 stdout（不超过 200 行）

project_context() {
  local ROOT="${1:-.}"
  local MAX_LINES=200
  # ... 生成目录结构、技术栈、测试列表、运行方式
}
```

### 5.2 command 输入组装逻辑

各 command.md 中的 agent 调度模板改为：

```
## Agent 调度

### 输入组装（模式感知）

1. 读取 STATUS.yaml 中的 mode
2. 生成项目上下文：`bash scripts/lib/context.sh`
3. 按模式决定前置文档：
   - full：要求前置文档存在
   - quick/fast/mvp：前置文档不存在时使用替代源
4. 组装 agent prompt = agent-prompt.md + 前置文档/替代源 + 项目上下文
```

### 5.3 各 command 具体修改

**spec.md**：
- full 模式：PRD.md（必须存在）+ 项目上下文
- quick/mvp 模式：BRAINSTORM.md（如有）或用户描述 + 项目上下文

**task.md**：
- full/quick 模式：SPEC.md（必须存在）+ 项目上下文
- fast 模式：用户描述 + 项目上下文（无需 SPEC.md）

**test.md**：
- 所有模式：SPEC.md + done_task_* 文件内容 + 项目上下文
- 修正"未完成 task"为"done_task_* 文件"

**devtest.md**：
- 所有模式：Done when + SPEC 相关片段 + 项目上下文

**fix.md**：
- 所有模式：issue 内容 + SPEC 相关片段 + 项目上下文

---

## 6. 清理项

### 删除冗余 hook 文件

以下文件的功能已被 `post-write.sh` 合并，不再被 hooks.json 引用：
- `scripts/hooks/check-doc-sync.sh`
- `scripts/hooks/check-phase-completion.sh`
- `scripts/hooks/check-task-completion.sh`
- `scripts/hooks/update-status.sh`

### 修正文档

- `test.md` 和 `test-agent.md`：`test_<功能>.py` → `test_<功能>.<ext>`（不硬编码语言）
- `marketplace.json`：description 中 "TEST → DONE" → "TEST → ITERATE"
- `test.md`：输入模板中"所有未完成 task 文件" → "所有 task 文件（含 done_task_*）"

---

## 7. 非功能需求

### 性能
- context.sh 执行时间 < 500ms（限定扫描深度和行数）
- 输出限制 200 行，大项目不会 token 爆炸

### 兼容性
- `tree` 命令不可用时 fallback 到 `find` 格式化
- 空项目（无 src/、无 tests/）时输出"空项目"而非报错

### 向后兼容
- full 模式行为不变（仍要求前置文档存在）
- 修改的是"文档不存在时的降级行为"，不影响正常流程

---

## 8. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 项目上下文过大超出 token 限制 | agent 输入被截断 | 硬限 200 行 + 只扫描顶层目录 |
| 替代输入源（用户描述）信息不足 | agent 产出质量下降 | command 提示用户补充必要信息 |
| 删除独立 hook 文件影响旧版引用 | 无影响，hooks.json 只引用 post-write.sh | 确认无外部引用后删除 |

---

## 9. 待定事项

1. context.sh 是否需要缓存（避免每次 agent 调度都重新扫描）？初步判断不需要——执行时间足够快。
2. "用户描述"如何传入 agent？由 command 执行者（主 agent）在组装 prompt 时将用户当前消息嵌入。
