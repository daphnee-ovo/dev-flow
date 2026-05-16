---
description: 选择开发模式 — 控制流程阶段
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# MODE — 开发模式选择

## 模式定义

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → done | 全新项目、需求模糊 |
| `quick` | spec → task → dev → test → done | 需求明确的功能开发 |
| `fast` | task → dev → test → done | 小改动、技术方案已知 |
| `mvp` | brainstorm → spec → dev | 快速验证想法、原型 |

## 执行步骤

### 1. 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.md" -path "*/*/STATUS.md" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

### 2. 设置模式

如果用户指定了模式（如 `/mode quick`），直接设置。否则询问。

### 3. 初始化

如果 `dev-doc/` 不存在：

```bash
mkdir -p dev-doc/{issue,session/{task,memory}}
```

### 4. 写入 STATUS.md

在 STATUS.md 中记录：

```markdown
开发模式：<mode>
```

### 5. 输出确认

```
[dev-flow] 模式已设置：<mode>
流程：<阶段列表，用 → 连接>
下一步：<第一个阶段对应的命令>
```

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
- STATUS 标记为 MVP

下一步：`/brainstorm`

## 命令可用性

| 命令 | full | quick | fast | mvp |
|------|:----:|:-----:|:----:|:---:|
| `/brainstorm` | ✓ | - | - | ✓ |
| `/prd` | ✓ | - | - | - |
| `/spec` | ✓ | ✓ | - | ✓ |
| `/task` | ✓ | ✓ | ✓ | - |
| `/devtest` | ✓ | ✓ | ✓ | - |
| `/fix` | ✓ | ✓ | ✓ | - |
| `/test` | ✓ | ✓ | ✓ | - |
| `/done` | ✓ | ✓ | ✓ | - |
| `/check` | ✓ | ✓ | ✓ | - |
| `/iterate` | ✓ | ✓ | ✓ | - |
| `/status` | ✓ | ✓ | ✓ | ✓ |

`-` 表示当前模式下此命令会提示"当前模式无需此步骤"。

## 模式切换

- 随时可通过 `/mode <新模式>` 切换
- 已有文档保留不删除
- 从低→高（如 fast → full）：提示用户补充缺失文档
- 从高→低（如 full → fast）：跳过后续不需要的阶段

## hooks 联动

`inject-context.sh` 读取 STATUS.md 中的模式字段，输出中体现：

```
[dev-flow] 当前阶段：DEV | 模式：quick | 任务进度：2/5
```

如果用户在当前模式下执行不可用的命令，提示：
```
[dev-flow] 当前模式为 fast，无需执行 /spec。如需完整流程请先 /mode full。
```
