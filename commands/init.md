---
description: 初始化 dev-flow 项目 — 扫描现状、创建/对齐 dev-doc、更新 agent 指令文件
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# INIT — 项目初始化

## 总则

`/init` 是 dev-flow 的入口命令。不管项目处于何种状态，执行后保证：
1. `dev-doc/` 目录结构符合规范
2. 所有文档格式通过校验
3. 项目级 agent 指令文件正确反映项目信息
4. 项目状态与实际一致

## 执行流程

### 阶段 1：环境探测 + 项目扫描

运行扫描获取项目信息：

```bash
dow scan
```

输出项目名、技术栈、命令、目录结构、git 状态、已有 dev-doc 等。

根据输出判断路径：
- 输出中 `dev_doc: none` → 路径 A（全新项目）
- 否则 → 路径 B（已有项目）

---

### 阶段 2A：全新项目初始化

1. 询问项目名称和开发模式（如扫描输出已有明确信息可跳过询问）
2. 执行初始化：
   ```bash
   dow init --name <项目名> --mode <mode>
   ```
   自动创建目录结构（dev-doc/{issue,task,archive}、tests、tmp）、写入 STATUS.yaml 和项目根目录 VERSION（多分支格式）。
   如果项目已有 `temp` 目录则沿用，不创建 `tmp`。
3. 跳到阶段 4

---

### 阶段 2B：已有项目对齐

#### 2B-1. 状态推断

根据脚本扫描结果判断实际阶段：

| 条件 | 推断阶段 |
|------|----------|
| 有代码 + 有通过的测试 + 有 TEST.md | DONE 或 TEST |
| 有代码 + 有 TASK.md 且部分完成 | DEV |
| 有 SPEC.md 但无/很少代码 | TASK 或 SPEC |
| 有 PRD.md 但无 SPEC.md | PRD → SPEC |
| 只有 README 或零散代码 | 根据模式确定初始阶段 |

各模式初始阶段对照：

| 模式 | 初始阶段 | 说明 |
|------|----------|------|
| `full` | PRD | 从需求定义开始 |
| `quick` | SPEC | 跳过需求探索 |
| `fast` | TASK | 直接拆任务 |
| `mvp` | SPEC | 快速验证路径（brainstorm → spec → dev） |

#### 2B-2. 向用户报告

输出扫描摘要，询问确认：
- 项目名称
- 开发模式
- 推断的阶段是否正确

---

### 阶段 2C：旧格式迁移（路径 B 时执行）

运行迁移检测脚本：

```bash
dow validate
```

脚本自动检测并迁移：
- `TASK.md` → `task/task_<today>_<seq>.md`（保留 `.bak`）
- `session/` → 提取摘要生成 `CHANGELOG.md`
- `STATUS.yaml` 中 `phase: MVP` → `phase: DEV`

如果输出 `status: no_migration_needed` 则跳过。

---

### 阶段 3：规范校验

运行校验脚本：

```bash
dow validate
```

脚本自动完成：
- 创建缺失目录
- 检查 STATUS.yaml 字段完整性
- 检查 task/ 文件格式和 Done when
- 检查 issue 文件命名和 frontmatter
- 补全 .gitignore

脚本输出报告，分三类：
- `auto_fixed`：已自动修复（目录创建、gitignore 等）
- `needs_confirm`：需要 agent 确认后处理（文件重命名）
- `warnings`：需要 agent 修复（缺失字段、格式错误）

**agent 只处理 `needs_confirm` 和 `warnings`**：
- `needs_confirm` → 询问用户确认后执行（如重命名 issue/task 文件）
- `warnings` → 直接修复（如补全 STATUS.yaml 缺失字段、补全 issue yaml 头）
  - `issue_nums_mismatch` → 直接修正 frontmatter 中的 nums 值为实际条目数
  - `issue_bad_item_format` → 修正条目格式为 `- [ ] I<N>：<标题>`
  - `issue_missing_required_fields` → 询问用户补充缺失字段或标记占位符
  - `issue_invalid_severity` → 修正为合法值 P0/P1/P2
- `auto_fixed` → 仅在最终报告中告知用户

**规范对照**：处理 `warnings` 时，agent 必须读取 `references/dev-doc/` 下的对应规范文档（如修复 issue 格式问题则读 ISSUE.md，修复 task 格式问题则读 TASK.md），确保修复内容符合规范定义。不要仅凭 warning 类型名推测正确格式。

---

### 阶段 4：更新 agent 指令文件

目标：让 agent 在后续会话中立即理解"怎么在这个项目干活"。

按当前运行环境优先选择：
- Codex：优先更新 `AGENTS.md`
- Claude Code：优先更新 `CLAUDE.md`
- 如果两个文件都存在，两个文件都更新
- 如果两个文件都不存在，创建当前运行环境对应的文件

#### 4-1. 写入内容

基于阶段 1 的扫描结果，写入：

```markdown
# <项目名>

<一句话描述>

## 开发

- 构建：`<build command>`
- 测试：`<test command>`
- 启动：`<dev server command>`

## 技术栈

<语言/框架/关键依赖>

## 项目结构

<主要目录及用途，不超过 10 行>

## 代码风格

<发现的风格约定，如无明确配置则省略此节>
```

#### 4-2. 更新规则

- 已有内容中非 dev-flow 产出的部分 → 保留不动
- 已有某节但信息过时 → 更新
- 目标文件不存在 → 整体生成
- **不写入 mode/phase** — 由 STATUS.yaml + hooks 管理
- 已有 `.cursorrules` / `.windsurfrules` → 读取并整合

---

### 阶段 5：输出确认

```
[dev-flow] 初始化完成
━━━━━━━━━━━━━━━━━━━━━━
项目名称：<name>
开发模式：<mode>
当前阶段：<phase>
迭代版本：v<N>
自动修复：<N> 项
需确认项：<N> 项（已处理）
agent 指令：已更新

下一步：<对应命令>
```

## 幂等性

- `/init` 可以重复执行
- 已有目录不会删除或覆盖内容
- STATUS.yaml 会按实际情况更新
- agent 指令文件只更新项目信息段落，不影响其他内容
- 每次执行都会重新扫描和校验
