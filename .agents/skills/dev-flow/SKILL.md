---
name: dev-flow
description: "项目全流程管理。命令：/init（项目初始化）、/brainstorm（头脑风暴）、/prd（需求探索）、/spec（技术规范）、/task（任务拆解）、/issue（创建 issue）、/devtest（例行测试）、/fix（自动修复 issue）、/test（完整测试）、/status（状态报告）、/check（文档同步检查）、/iterate（启动新迭代）、/mode（开发模式）。当用户提到创建项目、启动项目、初始化、项目状态、下一步、开始开发、新版本、迭代、头脑风暴、想法、模式时触发。"
---

# Dev-Flow：项目全流程管理

## Skill 加载确认

当此 skill 被触发时，输出：

```
[dev-flow] skill 已加载 | 当前阶段：<从 STATUS.yaml 读取，不存在则显示"新项目">
```

## 运行约定

- 当命令要求"独立 agent"或"subagent"时，使用 `spawn_agent`。这是 dev-flow 命令的显式子代理请求。
- 当命令要求更新项目级 agent 指令时，优先更新 `AGENTS.md`；如果 `CLAUDE.md` 也存在，保持两者同步。

## 流程总览

```
[头脑风暴(BRAINSTORM)] → 需求(PRD) → 规范(SPEC) → 任务(TASK) → 开发(DEV) → 测试(TEST) → 迭代(ITERATE) → 下一轮
      可选                                                     │                ↑
                                                               └── 例行TEST ──→│
```

> `/brainstorm` 在任何模式下都可用，始终是可选项。

## 命令映射

| 命令 | 阶段 | 角色 |
|------|------|------|
| `/init` | 初始化 | 创建 dev-doc、选择模式 |
| `/brainstorm` | PRD 前置 | 协作式设计探索 |
| `/prd` | PRD | 懂技术的高级产品经理 |
| `/spec` | SPEC | 资深架构师 |
| `/task` | TASK | 经验丰富的技术主管 |
| `/issue` | DEV/TEST | 手动创建 issue |
| `/devtest` | DEV（内循环） | 轻量 QA |
| `/fix` | DEV/TEST | 自动修复 issue |
| `/test` | TEST | 严格的 QA 工程师 |
| `/status` | 任意 | 状态报告 |
| `/check` | 任意 | 文档同步检查 |
| `/iterate` | 任意（满足交付条件时） | 归档 + commit & tag + bump 版本 |
| `/mode` | 任意 | 模式选择（full/quick/fast/mvp；audit 为自动触发） |

## 开发模式（/mode）

| 模式 | 流程 | 说明 |
|------|------|------|
| `full` | PRD → SPEC → TASK → DEV → TEST → ITERATE | 完整流程 |
| `quick` | SPEC → TASK → DEV → TEST → ITERATE | 跳过 PRD |
| `fast` | TASK → DEV → TEST → ITERATE | 最小设计 |
| `mvp` | SPEC → TASK → DEV → ITERATE | 跳过 TEST |
| `audit` | （自动触发，不可手动设置） | 见下方说明 |

### audit 模式

audit 模式用于处理非 DEV 阶段发现的紧急 issue，**不可手动设置**，仅由系统自动触发。

**触发条件**：在非 DEV 阶段创建 issue 文件（`issue/issue_*.md`）时，hook 自动将模式切换为 `audit/<原模式>`。

**行为**：
- mode 字段写为 `audit/<previous>`（如 `audit/quick`），保留原模式信息
- phase 强制设为 DEV，直接进入修复流程
- iterate 时跳过 task 完成度检查（因为 audit 轮只关注 issue 修复）
- inject-context 输出 audit 专用提示：`issue → /fix → /iterate 恢复原模式`

**恢复**：执行 `/iterate` 后自动恢复原模式（从 `audit/quick` 恢复为 `quick`），phase 重置为原模式的起始阶段。

## VERSION 文件机制

项目根目录下的 `VERSION` 文件记录当前语义化版本号（格式：`MAJOR.MINOR.PATCH`）。

**iterate 流程中的版本操作**：
1. 读取 `VERSION` 文件中的当前版本（如 `2.6.0`）
2. 以当前版本执行 `git commit` + `git tag v2.6.0`
3. 按 bump 类型（默认 minor）计算新版本（`2.7.0`）
4. 写入新版本到 `VERSION` 文件
5. 提交 `Start v2.7.0 iteration`

**支持的 bump 类型**：`major`（大版本）、`minor`（功能版本，默认）、`patch`（补丁版本）

## 角色隔离

不同阶段由独立 agent 执行，避免上下文互相干扰。每个 agent 只接收该阶段所需的最小输入。

| 阶段 | 执行方式 | 输入 |
|------|----------|------|
| PRD | 独立 agent | 用户描述 + BRAINSTORM.md（如有） |
| SPEC | 独立 agent | PRD.md（或 BRAINSTORM/描述按模式）+ 项目上下文 |
| TASK | 独立 agent | SPEC.md（或描述按 fast 模式）+ 项目上下文 |
| DEV | 主 agent 直接执行 | task/*.md + SPEC.md |
| TEST | 独立 agent | SPEC.md + task/*.md + 项目上下文 |

## DEV 阶段规则

开发阶段由主 agent 执行，遵循：
- **[BLOCKED] 阻断规则**：当 hook 输出包含 `[BLOCKED]` 时，禁止执行任何开发操作（编辑代码、运行命令），只允许执行 `/task`、`/issue`、`/iterate` 创建任务或 issue
- 只做 task/ 中列出的任务，不多不少
- 完成一个任务立即勾选，立即触发 `/devtest`
- 文档实时更新，不允许"稍后再改"
- 所有任务完成后自动进入 `/test`

## 目录结构

```
dev-doc/
├── STATUS.yaml
├── CHANGELOG.md
├── BRAINSTORM.md
├── PRD.md
├── SPEC.md
├── TEST.md
├── task/
│   ├── task_2026-05-15_1.md
│   └── done_task_2026-05-14_1.md
├── issue/
│   ├── issue_test_2026-05-15_1.md
│   └── closed_issue_test_2026-05-14_1.md
└── archive/
    └── v1-init/
```

## dow CLI

`scripts/bin/dow` 是 Rust 编写的统一调度器，所有 hook 和脚本化操作通过它执行。

| 子命令 | 作用 |
|--------|------|
| `dow status` | 读写 STATUS.yaml（`--phase`/`--mode`/`--exec-mode`/`--name`/`--field`） |
| `dow check` | 文档规范检查 |
| `dow iterate --topic <t> [-v minor] [--view]` | 迭代交付 |
| `dow scan` | 项目扫描 |
| `dow validate` | 校验 dev-doc 结构 |
| `dow doc --<type> [-n N] [--source X]` | 生成文档模板 |
| `dow devtest [--task <id>]` | 任务级测试 |
| `dow test [--file <x>]` | 全量测试 |
| `dow hooks context` | hook：注入上下文 |
| `dow hooks guard <file>` | hook：文件写入守护 |
| `dow hooks post-write <file>` | hook：写后联动 |
| `dow hooks save-changelog` | hook：保存 CHANGELOG |

默认 JSON 输出，`-H` 切换人类友好格式。编译：`bash dow/build.sh`。

## 格式 Schema 层（references/）

`references/dev-doc/` 是所有文档格式的唯一权威定义。agents 和 commands 通过引用获取格式，不内嵌模板。

| Schema 文件 | 定义内容 |
|-------------|----------|
| `TASK-FILE.md` | task 文件格式（priority/refs/files/done_when 等） |
| `SPEC-FILE.md` | SPEC.md 格式（含按 mode 降级规则） |
| `PRD-FILE.md` | PRD.md 格式（MoSCoW 优先级） |
| `ISSUE.md` | issue 文件格式 |
| `TEST.md` | 测试报告格式 |
| `STATUS.md` | STATUS.yaml schema |
| `BRAINSTORM-FILE.md` | brainstorm 文档格式 |
| `CHANGELOG.md` | changelog 格式 |

调度层（commands/*.md）负责在组装 subagent prompt 时读取 references/ 内容并拼入。

## 灵活性

- 小项目可合并阶段（如 PRD+SPEC 一步）
- 用户明确知道要什么时，不强制走完整流程
- 流程服务于项目，不是项目服务于流程
