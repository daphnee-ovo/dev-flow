---
description: 初始化 dev-flow 项目 — 扫描现状、创建/对齐 dev-doc、更新 agent 指令文件
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# INIT — 项目初始化

## 总则

`/init` 是 dev-flow 的入口命令。不管项目处于何种状态，执行后保证：
1. `dev-doc/` 目录结构符合规范
2. 所有文档格式符合 `dev-doc-spec.md`（STATUS/TASK/Issue/Session 格式、命名、yaml 头）
3. 项目级 agent 指令文件（Codex 的 `AGENTS.md` / Claude Code 的 `CLAUDE.md`）包含 dev-flow 规范
4. 项目状态与实际代码/文档一致（冲突已解决或记录为 issue）

## 执行流程

### 阶段 0：环境探测

```bash
# 检测 DOC_ROOT
BRANCH=$(git branch --show-current 2>/dev/null)
if [ -n "$BRANCH" ] && [ -f "dev-doc/$BRANCH/STATUS.md" ]; then
  DOC_ROOT="dev-doc/$BRANCH"
elif [ -f "dev-doc/STATUS.md" ]; then
  DOC_ROOT="dev-doc"
else
  DOC_ROOT=""
fi
```

根据 DOC_ROOT 是否存在，分两条路径：

---

### 路径 A：全新项目（无 dev-doc/STATUS.md）

1. 询问项目名称和开发模式（如参数已提供则跳过）
2. 创建目录结构：
   ```bash
   mkdir -p dev-doc/{issue,session/memory,archive}
   mkdir -p tests
   mkdir -p tmp
   ```
3. 写入 STATUS.md（初始阶段按模式确定）
4. 跳到「阶段 F：更新 agent 指令文件」

---

### 路径 B：已有项目（存在代码/文档）

#### B1. 项目扫描

**并行扫描以下内容：**

| 扫描项 | 方法 | 目的 |
|--------|------|------|
| 代码结构 | `find . -type f` 排除 node_modules/.git/tmp | 了解项目实际规模和模块 |
| 现有文档 | 读取 dev-doc/ 下所有 .md | 已有的 PRD/SPEC/TASK/TEST |
| Git 历史 | `git log --oneline -20` | 最近开发动态 |
| 测试现状 | 扫描 tests/ 目录 | 是否已有测试代码 |
| 依赖/配置 | package.json / pyproject.toml / go.mod 等 | 技术栈判断 |
| README | 读取 README.md | 项目描述 |

#### B2. 状态推断

根据扫描结果判断项目当前实际所处阶段：

| 条件 | 推断阶段 |
|------|----------|
| 有代码 + 有通过的测试 + 有 TEST.md | DONE 或 TEST |
| 有代码 + 有 TASK.md 且部分完成 | DEV |
| 有 SPEC.md 但无/很少代码 | TASK 或 SPEC |
| 有 PRD.md 但无 SPEC.md | PRD → SPEC |
| 只有 README 或零散代码 | 根据模式确定初始阶段 |

#### B3. 冲突检测

对比 dev-doc 文档与实际代码，检查：

| 检查项 | 冲突示例 |
|--------|----------|
| TASK.md 勾选 vs 代码实现 | 任务标记完成但对应功能不存在 |
| SPEC.md 接口定义 vs 实际代码 | 接口签名不一致 |
| STATUS.md 阶段 vs 实际进度 | 标记 DONE 但有未关闭 issue |
| issue/ 状态 vs 代码 | issue 标记 fix 但代码未改 |
| 测试文件 vs tests/ 实际 | TASK 中有测试任务但 tests/ 为空 |

#### B4. 向用户报告

输出扫描摘要和冲突列表，格式：

```
[dev-flow] 项目扫描完成
━━━━━━━━━━━━━━━━━━━━━━
项目名称：<从 README/package.json/STATUS.md 推断>
技术栈：<语言/框架>
代码规模：<文件数/行数概估>
现有文档：<列出已有的 dev-doc 文件>
推断阶段：<推断结果>

⚠ 发现 N 处冲突：
  1. [TASK vs 代码] 任务"实现用户认证"标记完成，但 src/auth/ 不存在
  2. [STATUS vs 进度] STATUS 为 DEV，但所有任务已完成
  ...

建议模式：<根据项目规模和现状推荐>
```

#### B5. 用户确认

询问用户：
- 确认/修改项目名称
- 确认/修改开发模式
- 对冲突的处理方式（逐条或批量）：
  - 以文档为准（更新代码侧标记）
  - 以代码为准（更新文档）
  - 跳过（保持现状，记为 issue）

#### B6. 对齐修正

根据用户选择：
- 补全缺失的 dev-doc 目录结构
- 修正 STATUS.md（阶段、模式、时间）
- 修正 TASK.md 勾选状态
- 将无法自动解决的冲突写入 `dev-doc/issue/issue_other_<YYYY-MM-DD>_<seq>.md`

---

### 阶段 D：文档规范校验（路径 A 和 B 都执行）

对 `dev-doc/` 下已有文件逐项校验，确保符合 `dev-doc-spec.md` 规范。

#### D1. 目录结构校验

| 检查项 | 预期 | 不符合时 |
|--------|------|----------|
| `dev-doc/issue/` 存在 | 必须 | 创建 |
| `dev-doc/session/memory/` 存在 | 必须 | 创建 |
| `dev-doc/archive/` 存在 | 必须 | 创建 |
| `tests/` 存在 | 必须 | 创建 |
| `tmp/` 存在 | 必须 | 创建 |
| 无临时文件散落在 dev-doc/ 或 src/ 中 | 应清洁 | 提醒用户移到 tmp/ |

#### D2. STATUS.md 格式校验

必须包含以下字段，缺失则补全：

```markdown
# 项目状态

- 项目名称：<name>
- 当前阶段：<PRD | SPEC | TASK | DEV | TEST | DONE | MVP>
- 开发模式：<full | quick | fast | mvp>
- 当前迭代：v<N>
- 更新时间：YYYY-MM-DD HH:MM
- 迭代启动时间：YYYY-MM-DD HH:MM
```

校验规则：
- 阶段值必须为合法枚举
- 模式值必须为 full/quick/fast/mvp 之一
- 时间格式正确
- 迭代版本为正整数

#### D3. TASK.md 格式校验

如果存在，检查：
- 每个任务行格式为 `- [ ] 任务名：描述` 或 `- [x] 任务名：描述`
- 每个任务下有 `  - Done when：<标准>`（缺失则标记警告）
- 无孤立文本（非任务行也非 Done when 行）

#### D4. Issue 文件校验

扫描 `dev-doc/issue/` 下所有文件：

| 检查项 | 规范 | 不符合时 |
|--------|------|----------|
| 文件名格式 | `[closed_]issue_<source>_<YYYY-MM-DD>_<seq>.md` | 提示重命名 |
| yaml 头完整性 | 必须含 source、modified_time、severity、status、task | 补全缺失字段 |
| source 值 | test / devtest / other | 修正非法值 |
| severity 值 | P0 / P1 / P2 | 修正非法值 |
| status 值 | exist / fix / closed | 修正非法值 |
| status 与文件名一致 | closed 状态 ↔ closed_ 前缀 | 统一（以 status 为准重命名） |
| 正文结构 | 含 # 标题、## 描述、## 发现位置 | 提示补全 |

#### D5. Session 文件校验

扫描 `dev-doc/session/` 下文件：

| 检查项 | 规范 | 不符合时 |
|--------|------|----------|
| 文件名格式 | `<3位数字>-<topic>.md` | 提示重命名 |
| 序号连续性 | 001, 002, 003... 不跳号 | 警告（不自动修） |

#### D6. 规范校验报告

输出校验结果：

```
[dev-flow] 文档规范校验
━━━━━━━━━━━━━━━━━━━━━━
✓ 目录结构完整
✓ STATUS.md 格式正确
⚠ TASK.md：3 个任务缺少 Done when
✗ issue/：2 个文件名不符合规范
  - bug-login.md → 应为 issue_other_2026-05-15_1.md
  - fix-api.closed.md → 应为 closed_issue_other_2026-05-15_1.md
✓ session/ 命名正确

自动修复 N 项，需用户确认 M 项。
```

#### D7. 自动修复 vs 用户确认

| 操作 | 自动执行 | 需确认 |
|------|----------|--------|
| 创建缺失目录 | ✓ | |
| 补全 STATUS.md 缺失字段 | ✓ | |
| 补全 issue yaml 头缺失字段 | ✓ | |
| 重命名不规范的 issue 文件 | | ✓ |
| 重命名不规范的 session 文件 | | ✓ |
| 移动散落的临时文件到 tmp/ | | ✓ |
| 修正 status/文件名不一致 | ✓（以 yaml status 为准） | |

---

### 阶段 E：TASK.md Done when 补全（可选）

如果 D3 发现有任务缺少 Done when，询问用户：
1. 现在逐个补全（交互式）
2. 跳过，后续开发时再补

---

### 阶段 F：更新 agent 指令文件（始终执行）

**不管路径 A 还是 B，最后都必须更新项目级 agent 指令文件。**

按当前运行环境优先选择：
- Codex：优先更新 `AGENTS.md`
- Claude Code：优先更新 `CLAUDE.md`
- 如果两个文件都存在，两个文件都更新，保持 dev-flow 段落一致
- 如果两个文件都不存在，创建当前运行环境对应的文件

确保目标文件包含以下 dev-flow 段落：

```markdown
## Dev-Flow 项目规范

- 流程管理：dev-flow 插件
- 命令：`/init` `/brainstorm` `/prd` `/spec` `/task` `/devtest` `/fix` `/test` `/done` `/status` `/check` `/iterate` `/mode`
- 文档目录：dev-doc/
- 当前模式：<mode>
- issue 命名：`[closed_]issue_<source>_<YYYY-MM-DD>_<seq>.md`
- 测试代码：统一放 tests/，命名 test_<模块>.py
- 临时文件：只能放项目 tmp/，禁止使用系统 /tmp/
- DEV 规则：完成任务 → 勾选 → /devtest → 测试写入 tests/
```

更新规则：
- 如果目标文件已有 `## Dev-Flow` 段落，替换该段落
- 如果没有，追加到文件末尾
- 保留目标文件中其他用户内容不动

### 阶段 G：.gitignore 检查

确保 `.gitignore` 包含：
```
tmp/
```
不包含则追加。

### 阶段 H：输出确认

```
[dev-flow] 初始化完成 ✓
━━━━━━━━━━━━━━━━━━━━━━
项目名称：<name>
开发模式：<mode>
当前阶段：<phase>
迭代版本：v<N>
文档目录：<DOC_ROOT>/
agent 指令文件：已更新 ✓

下一步：<对应命令>
```

## Agent 调度

路径 B 的扫描工作量较大，使用 subagent 并行。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：

```
description: "项目扫描 - 代码与文档分析"
prompt: `扫描当前项目，输出以下信息：
  1. 项目结构（主要目录和文件）
  2. 技术栈（语言、框架、依赖）
  3. 代码规模（文件数、预估行数）
  4. 已有 dev-doc/ 内容摘要
  5. tests/ 现状
  6. git log 最近 20 条
  7. README 摘要
  不要修改任何文件。`
```

冲突检测如果涉及多个模块，也可并行拆分。

## 幂等性

- `/init` 可以重复执行
- 已有目录不会删除或覆盖内容
- STATUS.md 会按实际情况更新
- agent 指令文件只更新 dev-flow 段落，不影响其他内容
- 每次执行都会重新扫描和对齐
