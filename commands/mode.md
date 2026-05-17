---
description: 选择开发模式 — 控制流程阶段
allowed-tools: Bash, Read, AskUserQuestion
---

# MODE — 开发模式选择

## 模式定义

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → done | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → done | 需求明确的功能开发 |
| `fast` | task → dev → test → done | 小改动、技术方案已知 |
| `mvp` | brainstorm → spec → dev | 快速验证想法、原型 |

## 执行方式

如果用户指定了模式（如 `/mode quick`），直接运行脚本：

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/mode.sh" <mode>
```

如果未指定模式，先询问用户选择，再运行脚本。

## 各模式规则

### full

**brainstorm → prd → spec → task → dev(devtest 循环) → test → done**

全流程，不跳过任何阶段。适合全新项目或需求模糊的大功能。

下一步：`/brainstorm`

### quick

**spec → task → dev(devtest 循环) → test → done**

跳过探索和需求定义，直接从技术方案开始。适合需求已经明确的功能。

下一步：`/spec`

### fast

**task → dev(devtest 循环) → test → done**

连技术方案都省了，直接拆任务开干。适合小改动、方案已知的场景。

下一步：`/task`

### mvp

**brainstorm → spec → dev**

最小验证路径。跳过正式需求文档、任务拆解、测试、交付。目标是最快出可运行的东西验证想法。

约束：
- 产出不直接进入生产
- 验证后如需正式开发，切换模式重新走流程
- 开发完成后使用 `/iterate` 进入下一轮（无需 `/done`）

下一步：`/brainstorm`

## 命令可用性

| 命令 | full | quick | fast | mvp |
|------|:----:|:-----:|:----:|:---:|
| `/brainstorm` | ✓ | ✓ | ✓ | ✓ |
| `/prd` | ✓ | - | - | - |
| `/spec` | ✓ | ✓ | - | ✓ |
| `/task` | ✓ | ✓ | ✓ | - |
| `/devtest` | ✓ | ✓ | ✓ | - |
| `/fix` | ✓ | ✓ | ✓ | - |
| `/test` | ✓ | ✓ | ✓ | - |
| `/done` | ✓ | ✓ | ✓ | - |
| `/check` | ✓ | ✓ | ✓ | - |
| `/iterate` | ✓ | ✓ | ✓ | ✓ |
| `/status` | ✓ | ✓ | ✓ | ✓ |

`-` 表示当前模式流程中不包含此步骤，执行时提示"当前模式无需此步骤"。

> 注：`/brainstorm` 是自由探索工具，不属于任何模式的必经阶段，但在所有模式下都可随时使用。

## 模式切换

- 随时可通过 `/mode <新模式>` 切换
- 已有文档保留不删除
- 从低→高（如 fast → full）：提示用户补充缺失文档
- 从高→低（如 full → fast）：跳过后续不需要的阶段

## hooks 联动

`inject-context.sh` 读取 STATUS.yaml 中的模式字段，输出中体现：

```
[dev-flow quick] STAGE: DEV | TASK: 2/5 | ISSUE: 0
```

如果用户在当前模式下执行不可用的命令，提示：
```
[dev-flow] 当前模式为 fast，无需执行 /spec。如需完整流程请先 /mode full。
```
