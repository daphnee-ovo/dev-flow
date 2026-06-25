# 头脑风暴记录 — dow CLI 重构：沉默原则 + 最小惊讶 + 资源模型

**日期**：2026-06-25

## 背景与目的

dow 当前存在几个设计问题：
1. 命令成功时输出冗余信息（违反沉默原则）
2. 部分命令语义不直觉（`dow doc` 承担多职责、`validate`/`check`/`fix` 职责模糊）
3. `.dev-doc/` 管理权分散——agent 可直接读写结构型文件，dow 只做部分 guard

目标：将 dow 重构为职责清晰的资源管理器，统一输出行为，让 agent 通过 dow 命令操作资源而非直接操作文件。

## 关键决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 沉默粒度 | 动作型成功且无额外信息→静默；`.dev-doc/` 内部操作不算额外信息 | 操作者发起动作已知结果，不需要确认；dow 内部存储实现对外不可见 |
| 命令体系 | 资源模型（noun + verb），消除 `dow doc` | task/issue/prd/spec 等各自独立为顶级命令，符合 gh/kubectl 惯例 |
| `.dev-doc/` 管理 | 文档型放开读写，结构型全链路走 dow | 文档型内容由 agent 生成（PRD/SPEC 等大段文本），强制 pipe 不实际；结构型字段明确，dow 可完全管理 |
| task 文件组织 | 按批次分文件，保持现有命名（task_001.md） | 人工可读，符合当前使用习惯 |
| issue 文件组织 | 保持一条一个文件，closed_ 前缀 | 零散产生，无批次概念 |
| schema 获取 | `dow <resource> schema` | `--help` 无法承载字段枚举和约束关系；每个资源域自包含格式定义 |
| 确认机制 | preview → token → confirm，三命令 token 前缀不同（ITR-/TRO-/IRO-） | 防止混用，confirm 时重传参数防止临时篡改 |
| exit code | 0=成功, 1=失败/warning, 2=用法错误 | 统一约定，warning 也非零（大声报错原则） |
| TEST.md | 砍掉 | 实际不用，`dow test` 直接跑脚本就够 |
| claim 无参 | 输出当前状态（含"无 claim"） | 查询型命令，空结果也是额外信息 |

## 设计方案

### 输出行为规则

**静默（exit 0，无 stdout）= 操作者发起动作 + 成功 + 无额外信息：**
- `dow hooks guard` — 放行
- `dow hooks post-write` / `dow hooks post-bash` — 联动完成
- `dow claim <ID>` / `dow claim --revoke` — 操作者知道自己做了什么
- `dow lint` 无问题时
- `dow lint --fix` 无需修复时
- `dow task done <ID>` / `dow issue close <ID>`
- `dow init`（`.dev-doc/` 内部操作不报告；`.dev-doc/` 之外的副作用才输出）

**输出 = 命令产生了操作者不知道的信息：**
- `dow lint` 有 warning/error — 具体问题是额外信息
- `dow lint --fix` 修了东西 — 修了什么是额外信息
- `dow init` 在 `.dev-doc/` 之外创建了文件
- `dow iterate --confirm` — 最终版本号、commit hash
- `dow test` — 测试结果
- `dow status` / `dow scan` / `dow task list` / `dow task show` 等查询型 — 纯信息输出
- `dow claim`（无参）— 当前状态

### 命令体系

```bash
# ── 资源命令 ──
dow task      create / list / show <ID> / done <ID> / reopen <ID> [--confirm TRO-xxx] / schema
dow issue     create / list / show <ID> / close <ID> / reopen <ID> [--confirm IRO-xxx] / schema
dow changelog list / add / schema
dow brainstorm create / schema
dow prd       create / show [--section ID] / schema
dow spec      create / show [--section ID] / schema

# ── 项目生命周期 ──
dow init --name <n> --mode <m>
dow status                          # 读
dow status set --phase/--mode/...   # 写
dow iterate --topic <t> --type <T> [--files ...] [-v patch] [--confirm ITR-xxx]
dow version [--bump patch|minor|major]

# ── 操作 ──
dow test [--task <ID>]
dow lint [--fix]
dow scan
dow claim [IDs] [--revoke]
dow rollback [--list | --version <v>]

# ── 基础设施 ──
dow archive ...
dow hooks ...
dow setup
```

### `.dev-doc/` 管理权模型

| 文件类型 | 创建 | 读取 | 编辑 |
|----------|------|------|------|
| 文档型（PRD.md / SPEC.md / BRAINSTORM.md） | dow 创建 | agent 直接 Read | agent 直接 Write/Edit |
| 结构型（task_*.md / issue_*.md / STATUS.yaml / CHANGELOG.md） | dow 命令 | dow 命令（list/show），指令软约束 agent 不直接 Read | dow 命令 |

guard hook 逻辑：
- Write/Edit `.dev-doc/` 下文件：
  - 文件不存在 → 拦截，提示用 dow 创建
  - 文件已存在 + 文档型 → 放行
  - 文件已存在 + 结构型 → 拦截，提示用 dow 命令
- Read：无 hook，靠注入指令软约束

### task/issue 写入方式

支持两种输入：
1. **flags**（简单场景）：`dow task create --title "..." --type feat --priority P0 --done-when "..."`
2. **stdin JSON**（批量/复杂场景）：`echo '<json>' | dow task create`（单对象或数组）

dow 自动检测 stdin 是否有数据来判断输入模式。

### 确认机制（iterate / reopen）

三步流程：
1. 首次执行 = preview，输出影响说明 + 生成 token
2. 操作者确认后重传参数 + token
3. dow 执行并校验 token 对应关系

```bash
dow iterate --topic "优化" --type refactor
# → 输出预览 + ITR-a3f7c2

dow iterate --topic "优化" --type refactor --confirm ITR-a3f7c2
# → 执行
```

Token 前缀：ITR-（iterate）、TRO-（task reopen）、IRO-（issue reopen）。confirm 时参数以本次传入为准，dow 内部对比 preview 快照若有变更则输出 diff。

### Breaking Changes 清单

| 现有命令 | 变更 |
|----------|------|
| `dow doc <type>` | 消除，拆入各资源命令的 `create` |
| `dow doc --json/--md` | 消除，各资源 `schema` 替代 |
| `dow check` + `dow validate` + `dow fix` | 合并为 `dow lint [--fix]` |
| `dow devtest` | 消除，`dow test --task <ID>` 替代 |
| `dow status --phase X`（写） | 改为 `dow status set --phase X` |
| `dow issue --list` | 改为 `dow issue list` |
| `dow task`（无子命令） | 新增 `create/list/show/done/reopen/schema` |
| TEST.md | 砍掉，不再生成 |

## 约束与边界

- 不改变 `dow hooks context` 的输出格式（hook 协议向后兼容）
- 不改变 archive 子命令体系（已稳定）
- 人工仍可直接查看 `.dev-doc/` 文件（只约束 agent 不直接操作结构型）
- 文件层面的组织（命名、前缀、路径）是 dow 内部实现，可后续独立优化

## 下一步

建议直接进入 `/spec` — 需求已明确，直接产出技术规范（命令签名、数据结构、模块拆分）。
