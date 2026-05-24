---
description: 开发中例行测试 — 任务级双重验证（DEV 阶段内循环）
allowed-tools: Agent, Bash, Read, Write, Edit
---

# DEV-TEST — 例行测试（双重 Review）

## 前置检查

1. 确认 STATUS 为 DEV
2. 确认 `task/` 目录中有已勾选 `[x]` 的任务
3. 生成项目上下文：`bash "${CLAUDE_PLUGIN_ROOT}/scripts/lib/context.sh"`

## 命令接口

```
/devtest                    # 默认逐步模式
/devtest --continuous       # 切换为连续执行模式（写入 STATUS.yaml）
/devtest --step             # 切换回逐步模式
```

切换操作更新 STATUS.yaml 的 `exec_mode` 字段（step/continuous）。

## 触发时机

- 每完成一个任务后**必须**执行（hooks 会强制提醒）
- STATUS 保持 DEV，不切换阶段

## 双重 Review 流程

```
任务完成 → [Round 1: Spec Compliance] → PASS/FAIL
                                             ↓ (仅 PASS 时)
                                    [Round 2: Code Quality] → PASS/WARN/FAIL
                                             ↓
                                    综合判定 → 状态返回
```

### 综合判定规则

| Round 1 | Round 2 | 综合状态 | 行为 |
|---------|---------|----------|------|
| PASS | PASS | DONE | 标记任务完成 |
| PASS | 仅 WARN | DONE_WITH_CONCERNS | 标记完成，concerns 写入 issue (P2) |
| PASS | FAIL | BLOCKED | 写 issue，要求修复 |
| FAIL | (跳过) | BLOCKED | 写 issue，要求修复 |

## Agent 调度 — Round 1: Spec Compliance

**启动独立 Spec Compliance 子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。**

```
description: "Spec Compliance Review - 验证任务 <任务名>"
prompt: `你是一名规范验证工程师。对照 SPEC 验证实现的功能正确性。

## 验证目标

任务：<从 task/ 文件中摘取刚完成的任务名>
Done when：<该任务的 Done when 标准，原文>

## 规范参考

<仅从 SPEC.md 中摘取与该任务相关的部分，如接口定义、数据模型>

## 项目上下文

<执行 scripts/lib/context.sh 的输出，原样粘贴>

## 验证要求

1. 逐条对照 Done when 验证
2. 检查实现是否符合 SPEC 定义的接口/格式/行为
3. 不评价代码质量（那是下一轮的事）
4. 测试代码写入 tests/ 目录
5. 只验证这一个任务，不扩大范围

## 输出格式

## Spec Compliance Review
- 验证项 1：PASS/FAIL — 说明
- 验证项 2：PASS/FAIL — 说明
...
结论：PASS / FAIL
原因：<如果 FAIL，精确描述问题和复现步骤>

---
STATUS: DONE / BLOCKED
DETAIL: <如果 BLOCKED，说明原因>
---

## 未通过时写入 Issue

如果结论为 FAIL，写入 issue 文件：

路径：<DOC_ROOT>/issue/issue_devtest_<YYYY-MM-DD>_<seq>.md

格式：
\`\`\`markdown
---
source: devtest
nums: <问题数量>
---

- [ ] I1：<问题标题>
  - severity: P0/P1
  - location: <文件路径>
  - description: <问题描述>
  - reproduce: <复现步骤>
  - fix: <修复建议>
\`\`\`

## 禁止

- 不要阅读无关的历史文件
- 不要验证其他任务
- 不要评价代码质量（代码审查是 Round 2 的职责）
- 禁止使用系统 /tmp/，临时文件只能放项目 tmp/ 下`
```

## Agent 调度 — Round 2: Code Quality

**仅当 Round 1 结论为 PASS 时启动。启动独立 Code Quality 子代理。**

```
description: "Code Quality Review - 验证任务 <任务名>"
prompt: `你是一名代码审查专家。检查代码质量。

## 审查范围

任务：<任务名>
涉及文件：<该任务修改的文件列表>

## 上一轮结论

<Round 1 Spec Compliance 的输出结果>

## 审查维度

1. 可读性：命名、注释、结构清晰度
2. 可维护性：模块化、职责单一、扩展性
3. 性能：明显的性能问题（不过度优化）
4. 安全：输入校验、路径穿越、注入风险

## 项目上下文

<执行 scripts/lib/context.sh 的输出，原样粘贴>

## 评分标准

- PASS：无问题
- WARN：有改进空间但不阻断（记录但不要求修复）
- FAIL：必须修复的质量问题

## 输出格式

## Code Quality Review
- 可读性：PASS/WARN/FAIL — 说明
- 可维护性：PASS/WARN/FAIL — 说明
- 性能：PASS/WARN/FAIL — 说明
- 安全：PASS/WARN/FAIL — 说明
结论：PASS / FAIL
建议：<如果有 WARN/FAIL，具体修改建议>

---
STATUS: DONE / DONE_WITH_CONCERNS / BLOCKED
DETAIL: <说明>
---

## 禁止

- 不要重复验证功能正确性（Round 1 已覆盖）
- 不要建议新功能
- 不要因为代码风格偏好报 FAIL（仅报 WARN）
- 禁止使用系统 /tmp/，临时文件只能放项目 tmp/ 下`
```

## Subagent 状态返回协议

subagent 在输出末尾必须包含状态块：

```
---
STATUS: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
DETAIL: <可选，说明原因/疑虑/需要的信息>
---
```

### Controller 行为

| 状态 | Controller 行为 |
|------|-----------------|
| `DONE` | 标记 task 完成，推进下一个 |
| `DONE_WITH_CONCERNS` | 标记完成，将 concerns 写入 issue（severity: P2），推进 |
| `NEEDS_CONTEXT` | 暂停，向用户展示 subagent 的问题并请求补充信息 |
| `BLOCKED` | 暂停，写 issue，提示用户处理 |

**向后兼容**：如果 subagent 输出中无 STATUS 行，视为 DONE。

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| 该任务的描述和 Done when | 其他任务的信息 |
| SPEC.md 中相关部分（非全文） | 开发过程对话历史 |
| 项目上下文（context.sh 输出） | 无关历史记录 |
| Round 1 结论（传入 Round 2） | PRD.md |

## 连续执行模式

读取 STATUS.yaml 的 `exec_mode` 字段。如果为 `continuous`：

### 推进规则

1. 当前 task 验证通过（STATUS: DONE）→ 自动找到下一个未完成 task（按 level 优先级 P0>P1>P2，同级按文件顺序）
2. 当前 task DONE_WITH_CONCERNS → 写 issue 后继续推进下一个 task
3. 当前 task NEEDS_CONTEXT → 停顿，向用户展示需要的信息，等待输入
4. 当前 task BLOCKED → 停顿，写 issue，提示用户处理
5. 所有 task 完成 → 停顿，提示执行 /test

### 停顿条件（必须停下来等用户）

- NEEDS_CONTEXT 状态
- BLOCKED 状态
- 所有任务完成
- 遇到无法解决的依赖冲突

### 连续推进时的输出

每完成一个 task，输出一行摘要：
```
[continuous] T3 DONE — <任务名>
[continuous] T4 开始...
```

## 结果处理

- **DONE** → 保持 task 文件的 `[x]`，继续下一个任务
- **DONE_WITH_CONCERNS** → 保持 `[x]`，concerns 写入 issue，继续
- **BLOCKED** → 取消勾选（改回 `[ ]`），issue 文件已写入，可用 `/fix` 修复后重新 /devtest
- **NEEDS_CONTEXT** → 保持 `[x]`，暂停等待用户提供信息

## 为什么双重 Review

单轮 QA 容易混淆"功能是否正确"和"代码是否好"。分离关注点：
- Round 1 只看"做对了没"（对照 SPEC）
- Round 2 只看"做好了没"（代码质量）

这种分离确保功能正确性不被代码风格讨论稀释，也确保代码质量不被"反正功能对了"掩盖。
