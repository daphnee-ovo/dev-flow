# 技术规范（SPEC）

## 1. 概述

本次迭代目标：借鉴 Superpowers 项目工程实践，对 dev-flow 进行四项增强，提升开发阶段的自动化程度和质量保障能力。

核心变更：
1. **双重 Review** — /devtest 从单轮 QA 改为 Spec Compliance + Code Quality 两轮
2. **连续执行模式** — DEV 阶段支持自动推进 task（opt-in），无需逐个确认
3. **模型分级建议** — task 文件增加 model hint 字段，指导 subagent 模型选择
4. **Implementer 交互支持** — subagent 支持多状态返回（DONE / NEEDS_CONTEXT / BLOCKED 等）

设计原则：
- 纯文档驱动，所有逻辑在 markdown 命令文件中定义
- 向后兼容，新功能均为 opt-in 或字段可选
- 不引入新的外部依赖

---

## 2. 架构设计

### 系统架构

```
用户 → command.md（调度逻辑） → Agent subagent（执行）
                                       ↓
                              状态返回（多状态协议）
                                       ↓
                            controller 决策（推进/暂停/补充）
```

四项增强对应的架构层变更：

| 增强 | 影响层 | 变更性质 |
|------|--------|----------|
| 双重 Review | command 调度层（devtest.md） | 单 agent → 双 agent 串行 |
| 连续执行 | command 调度层 + hook 层 | 新增 flow 控制协议 |
| 模型分级 | agent 产出格式 + command 调度参考 | 字段扩展（向后兼容） |
| 交互支持 | agent 返回协议 + controller 决策逻辑 | 新协议定义 |

### 模块划分

| 模块 | 职责 | 涉及文件 |
|------|------|----------|
| devtest 命令 | 双重 Review 调度 | `commands/devtest.md` |
| task 命令 | model hint 产出指导 | `commands/task.md` |
| task agent | model hint 字段定义 | `agents/task-agent.md` |
| DEV 流程控制 | 连续执行 + 状态协议 | `commands/devtest.md`、`scripts/hooks/inject-context.sh` |
| 状态协议 | subagent 返回规范 | `commands/devtest.md`（内嵌定义） |

### 目录结构

本次迭代不新增文件，仅修改现有文件：

```
commands/
├── devtest.md          # 主要修改：双重 Review + 连续执行 + 状态协议
├── task.md             # 小修改：传递 model hint 指令给 task agent
agents/
├── task-agent.md       # 修改：增加 model hint 字段定义和判断标准
scripts/hooks/
├── inject-context.sh   # 小修改：展示连续执行模式状态
├── post-write.sh       # 小修改：连续执行模式下的 devtest 触发提示调整
```

---

## 3. 技术选型

| 领域 | 选择 | 理由 | 备选方案 |
|------|------|------|----------|
| 双重 Review 调度 | 串行双 agent（两次 Agent 调用） | 两轮验证职责不同（功能正确性 vs 代码质量），隔离关注点；第二轮可以参考第一轮结论避免重复报告 | 单 agent 合并两轮（prompt 过长导致关注点模糊）；并行双 agent（无法互相参考） |
| 连续执行控制 | STATUS.yaml 增加 `exec_mode` 字段 + devtest.md 中定义推进逻辑 | 复用已有的状态文件机制，无需新文件；command.md 本身就是调度逻辑载体 | 新增独立配置文件（过度设计）；hook 中硬编码（不透明） |
| 模型分级 | task 文件中每个 task 增加 `model` 字段 | 跟随任务定义，无需额外映射；字段可选保证向后兼容 | 独立 model-map 文件（增加维护负担）；仅在 SPEC 中标注（粒度不够） |
| 状态协议 | 在 devtest.md 的 agent prompt 中定义输出格式约束 | agent 返回格式由 prompt 定义是 LLM 应用标准做法；无需代码解析 | 结构化 JSON 返回（markdown 系统中引入 JSON 不一致）；在独立文件定义协议（分散注意力） |

---

## 4. 数据模型

### 4.1 STATUS.yaml 扩展

```yaml
name: dev-flow
phase: DEV
mode: quick
exec_mode: step          # 新增：step（逐步，默认）| continuous（连续）
updated: 2026-05-24 21:31
started: 2026-05-24 21:31
```

`exec_mode` 字段规则：
- 默认值 `step`（向后兼容，不写等同 step）
- 仅在 DEV 阶段有意义，其他阶段忽略
- 用户通过 `/devtest --continuous` 或 `/devtest --step` 切换

### 4.2 Task 文件格式扩展

```markdown
- [ ] T1：实现双重 Review 的 Spec Compliance 轮
  - level: P0
  - model: standard           # 新增字段
  - details：修改 devtest.md，增加第一轮 Spec Compliance subagent
  - depends on：无
  - Done when：devtest 命令中包含 Spec Compliance agent 模板，prompt 包含 SPEC 对照验证指令
```

`model` 字段值定义：

| 值 | 含义 | 典型场景 | 
|------|------|----------|
| `cheap` | 机械性实现，规范明确 | 修改 1-2 个文件、格式调整、字段增删 |
| `standard` | 需要集成判断 | 多文件协调、接口对接、逻辑分支 |
| `capable` | 需要设计决策 | 架构变更、复杂调试、权衡取舍 |

判断标准（写入 task-agent.md）：
- 影响文件数 <=2 且有明确模板/规范可循 → `cheap`
- 影响文件 3-5 个或需要理解模块交互 → `standard`
- 需要做出 SPEC 中未明确的设计决策或涉及架构调整 → `capable`

字段可选：不填时 controller 默认按 `standard` 处理。

### 4.3 Subagent 状态返回协议

subagent 在输出末尾必须包含状态行：

```
---
STATUS: DONE
```

可选状态值：

| 状态 | 含义 | controller 行为 |
|------|------|-----------------|
| `DONE` | 任务完成，验证通过 | 标记 task 完成，推进下一个 |
| `DONE_WITH_CONCERNS` | 完成但有疑虑 | 标记完成，但将 concerns 写入 issue（severity: P2） |
| `NEEDS_CONTEXT` | 缺少信息无法继续 | 暂停连续执行，向用户展示 subagent 的问题并请求补充 |
| `BLOCKED` | 无法完成，需升级 | 暂停连续执行，标记 task 为 blocked，写 issue |

状态行格式：
```
---
STATUS: <状态值>
DETAIL: <可选，说明原因/疑虑/需要的信息>
---
```

### 4.4 双重 Review 数据流

```
输入 → [Round 1: Spec Compliance] → 结论(PASS/FAIL)
                                          ↓
                                    [Round 2: Code Quality] → 结论(PASS/FAIL)
                                          ↓
                                    综合判定 → 状态返回
```

Round 1 输出格式：
```
## Spec Compliance Review
- 验证项 1：PASS/FAIL — 说明
- 验证项 2：PASS/FAIL — 说明
结论：PASS / FAIL
```

Round 2 输出格式：
```
## Code Quality Review
- 可读性：PASS/WARN/FAIL — 说明
- 可维护性：PASS/WARN/FAIL — 说明
- 性能：PASS/WARN/FAIL — 说明
- 安全：PASS/WARN/FAIL — 说明
结论：PASS / FAIL
```

综合判定规则：
- 两轮均 PASS → `DONE`
- Round 1 FAIL → 必须修复（写 issue，status 为 BLOCKED）
- Round 1 PASS + Round 2 FAIL → 写 issue（severity 按问题定）
- Round 2 仅 WARN → `DONE_WITH_CONCERNS`

---

## 5. 接口设计

### 5.1 devtest.md 命令接口

```
/devtest                    # 默认逐步模式
/devtest --continuous       # 切换为连续执行模式（本次及后续）
/devtest --step             # 切换回逐步模式
```

切换操作更新 STATUS.yaml 的 `exec_mode` 字段。

### 5.2 devtest.md 内部 Agent 调度模板（修改后）

**Round 1 - Spec Compliance Agent:**

```
description: "Spec Compliance Review - 验证任务 <任务名>"
prompt: `你是一名规范验证工程师。对照 SPEC 验证实现的功能正确性。

## 验证目标

任务：<任务名>
Done when：<完成标准>

## 规范参考

<SPEC.md 中与该任务相关的部分>

## 项目上下文

<context.sh 输出>

## 验证要求

1. 逐条对照 Done when 验证
2. 检查实现是否符合 SPEC 定义的接口/格式/行为
3. 不评价代码质量（那是下一轮的事）
4. 测试代码写入 tests/ 目录

## 输出格式

## Spec Compliance Review
- 验证项 1：PASS/FAIL — 说明
...
结论：PASS / FAIL
原因：<如果 FAIL，精确描述>

---
STATUS: DONE / BLOCKED
DETAIL: <如果 BLOCKED，说明原因>
---`
```

**Round 2 - Code Quality Agent:**

```
description: "Code Quality Review - 验证任务 <任务名>"
prompt: `你是一名代码审查专家。检查代码质量。

## 审查范围

任务：<任务名>
涉及文件：<该任务修改的文件列表>

## 上一轮结论

<Round 1 的输出结果>

## 审查维度

1. 可读性：命名、注释、结构清晰度
2. 可维护性：模块化、职责单一、扩展性
3. 性能：明显的性能问题（不过度优化）
4. 安全：输入校验、路径穿越、注入风险

## 项目上下文

<context.sh 输出>

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
---`
```

### 5.3 连续执行模式的 Controller 逻辑

在 `devtest.md` 末尾增加连续执行指令：

```markdown
## 连续执行模式

读取 STATUS.yaml 的 exec_mode 字段。如果为 continuous：

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
[continuous] T3 DONE — <任务名>（耗时约 Xmin）
[continuous] T4 开始...
```
```

### 5.4 task-agent.md 的 model 字段指令

在 task-agent.md 的 "Task 文件结构" 部分增加：

```markdown
- [ ] T1：<任务名称>
  - level: P0
  - model: cheap | standard | capable    # 新增
  - details：具体做什么
  - depends on：无
  - Done when：验收标准
```

### 5.5 inject-context.sh 状态展示扩展

DEV 阶段输出增加执行模式标识：

```
[dev-flow quick] v2.2(no-tag) | STAGE: DEV[continuous] | TASK: 2/5 | ISSUE: 0
```

当 `exec_mode` 为 `continuous` 时在 STAGE 后标注 `[continuous]`；`step` 模式不标注（默认行为）。

---

## 6. 非功能需求

### 性能

- 双重 Review 引入额外一次 agent 调用，单 task 验证时间约增加 1 倍
- 连续执行模式下连续 agent 调用无额外启动开销（串行执行）
- model hint 不影响执行性能（仅建议性元数据）

### 安全

- 状态协议中的 STATUS 行由 controller（主 agent）解析，不执行其中内容
- 连续执行模式下 BLOCKED/NEEDS_CONTEXT 是硬停顿点，防止 agent 在错误方向上持续执行

### 兼容性

- `exec_mode` 字段可选，不存在时等同 `step`
- `model` 字段可选，不存在时 controller 默认按 `standard` 处理
- 双重 Review 是 devtest 的固定行为（不可选），但不影响已有的 test 命令（/test 保持不变）
- 状态协议是新增约定，旧的 subagent（不输出 STATUS 行）视为 DONE

### 向后兼容性详细说明

| 变更点 | 向后兼容策略 |
|--------|-------------|
| STATUS.yaml exec_mode | 字段不存在 = step（现有行为） |
| task 文件 model 字段 | 字段不存在 = standard（现有行为） |
| devtest 双重 Review | 始终启用，但不改变通过/不通过的最终语义 |
| subagent 状态协议 | 无 STATUS 行 = DONE（兼容旧行为） |

---

## 7. 风险与缓解

| 风险 | 影响 | 概率 | 缓解 |
|------|------|------|------|
| 双重 Review 过于严格导致频繁 FAIL | 开发流程变慢，用户体验下降 | 中 | Code Quality 轮的 WARN 不视为 FAIL；只有明确的质量问题才 FAIL |
| 连续执行模式下 agent 连续犯错 | 多个 task 产出有问题 | 低 | 每个 task 仍然经过双重 Review 把关；BLOCKED 强制停顿 |
| model hint 不准确 | 用了 cheap 模型做 capable 任务 | 中 | model hint 是建议性的，controller 可根据实际复杂度覆盖；明确标注"建议性" |
| NEEDS_CONTEXT 被滥用 | subagent 频繁请求信息导致流程碎片化 | 低 | 在 prompt 中明确只有真正缺少无法推断的信息才返回此状态 |
| inject-context.sh 解析新字段出错 | 状态显示不正确 | 低 | exec_mode 不存在时跳过展示，不影响功能 |

---

## 8. 待定事项

1. **Code Quality Review 的评判标准细化** — 当前定义了四个维度（可读性/可维护性/性能/安全），是否需要为不同项目类型（脚本项目 vs Web 应用）定制权重？初步判断：不需要，保持通用即可。
2. **连续执行模式的切换粒度** — 当前设计为全局切换（影响整个 DEV 阶段）。是否需要支持"本次执行连续，下次恢复逐步"？初步判断：全局切换够用，用户随时可 `--step` 切回。
3. **model hint 与实际模型的映射** — hint 到具体模型名称的映射由运行时环境决定（Claude Code vs Codex 可能可用模型不同）。本次只定义 hint 语义，不定义具体模型映射表。
