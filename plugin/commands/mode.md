---
description: 选择开发模式 — 控制流程阶段
allowed-tools: Bash, Read, AskUserQuestion
---

# MODE — 开发模式选择

## 模式定义

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | prd → spec → task → dev → test → iterate | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → iterate | 需求明确的功能开发 |
| `fast` | task → dev → test → iterate | 小改动、技术方案已知 |
| `mvp` | spec → task → dev → iterate | 快速验证，跳过 TEST |
| `audit` | （自动触发，不可手动设置） | 非 DEV 阶段创建 issue 时自动进入 |

## 执行方式

如果用户指定了模式（如 `/mode quick`），直接运行脚本：

```bash
dow status --mode <mode>
```

如果未指定模式，先询问用户选择，再运行脚本。

## 各模式规则

### full

**prd → spec → task → dev(devtest 循环) → test → iterate**

全流程，不跳过任何阶段。适合全新项目或需求模糊的大功能。brainstorm 可选前置。

约束：
- 所有任务（不分优先级）必须全部完成才能 iterate
- 每个阶段文档必须满足 phase-completion 检查标准

下一步：`/prd`（或先 `/brainstorm`）

### quick

**spec → task → dev(devtest 循环) → test → iterate**

跳过探索和需求定义，直接从技术方案开始。适合需求已经明确的功能。

下一步：`/spec`

### fast

**task → dev(devtest 循环) → test → iterate**

连技术方案都省了，直接拆任务开干。适合小改动、方案已知的场景。

约束：
- 所有任务（不分优先级）必须全部完成才能 iterate
- P0/P1 任务必须实现，P2 可标记为"推迟到下一迭代"但不可删除

下一步：`/task`

### mvp

**spec → task → dev → iterate**

最小验证路径。跳过 PRD 和 TEST，直接从规范到交付。目标是最快出可运行的东西验证想法。

约束：
- 产出不直接进入生产
- 验证后如需正式开发，切换模式重新走流程
- 开发完成后使用 `/iterate` 进入下一轮

下一步：`/spec`（或先 `/brainstorm`）

## 命令可用性

| 命令 | full | quick | fast | mvp |
|------|:----:|:-----:|:----:|:---:|
| `/brainstorm` | ✓ | ✓ | ✓ | ✓ |
| `/prd` | ✓ | - | - | - |
| `/spec` | ✓ | ✓ | - | ✓ |
| `/task` | ✓ | ✓ | ✓ | ✓ |
| `/devtest` | ✓ | ✓ | ✓ | ✓ |
| `/fix` | ✓ | ✓ | ✓ | ✓ |
| `/test` | ✓ | ✓ | ✓ | - |
| `/check` | ✓ | ✓ | ✓ | ✓ |
| `/iterate` | ✓ | ✓ | ✓ | ✓ |
| `/status` | ✓ | ✓ | ✓ | ✓ |

`-` 表示当前模式流程中不包含此步骤，执行时提示"当前模式无需此步骤"。

> 注：`/brainstorm` 是自由探索工具，不属于任何模式的必经阶段，但在所有模式下都可随时使用。

## audit 模式

audit 模式是自动触发的临时覆盖模式：
- 当非 DEV 阶段创建 issue 时自动进入
- 格式为 `audit/<原模式>`（如 `audit/quick`）
- iterate 后自动恢复为原模式
- 不可通过 `/mode audit` 手动设置

## 模式切换

- 随时可通过 `/mode <新模式>` 切换
- 已有文档保留不删除
- 从低→高（如 fast → full）：提示用户补充缺失文档
- 从高→低（如 full → fast）：跳过后续不需要的阶段
