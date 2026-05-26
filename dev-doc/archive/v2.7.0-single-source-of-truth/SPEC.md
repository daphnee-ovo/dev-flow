# SPEC: 单一真相源重构

## Goal

消除格式定义散布在 agents/commands/references/skills 多处的现状，建立 `references/` 为唯一 schema 层，其他层通过引用获取格式规范，不再内嵌复制。

## Scope

### In
- 升级 `references/` 为权威 schema 目录（对齐到 agent 中的最新格式）
- agents/*.md 中的格式定义改为引用 references/
- commands/*.md 中的格式定义改为引用 references/
- SKILL.md (x3) 同步到最新状态（audit mode、VERSION 等）
- `scripts/init/validate.sh` 对齐 references/ 中的 schema
- `references/dev-flow-spec.md` 更新到当前实际状态

### Out
- 不改变任何运行时行为（纯文档/规范层重构）
- 不新增 schema 解析工具（人类可读 markdown 即 schema）
- 不改变目录结构或文件命名

## Requirements Trace

| Req | AC | Notes |
| --- | --- | --- |
| user-request: 解决多处定义不一致 | SPEC-AC-001 | 格式定义归一 |
| user-request: agent 应该遵循哪个 | SPEC-AC-002 | 引用链明确 |
| user-request: SKILL.md 没有更新 | SPEC-AC-003 | 同步最新特性 |

## Design

### 层次模型

```
references/          ← 唯一格式 schema（权威定义）
    ├── dev-doc/
    │   ├── TASK-FILE.md      # task 文件格式
    │   ├── ISSUE.md          # issue 文件格式
    │   ├── SPEC-FILE.md      # SPEC 文件格式（新增）
    │   └── STATUS.md         # STATUS.yaml schema（新增）
    └── dev-flow-spec.md      # 流程规范总览

agents/*.md          ← 角色 + 行为（引用 references/ 获取格式）
commands/*.md        ← 调度流程（引用 references/ 获取格式）
skills/*/SKILL.md    ← 插件概览（引用 references/ 获取格式）
scripts/init/        ← 校验逻辑（基于 references/ 定义实现）
```

### 引用方式

agents 和 commands 中不再内嵌完整格式模板，改为：

```markdown
## 输出格式

遵循 `references/dev-doc/TASK-FILE.md` 定义的格式。
```

对于 agent prompt 需要完整传入格式（因为 subagent 无法自己读文件）的情况，由**调度层（commands/*.md）**负责在组装 prompt 时读取 references/ 内容并拼入。agent 文件本身只声明"格式来源"。

### 格式对齐方向

以 `agents/task-agent.md` 中的当前格式为最新真相，反向更新 `references/dev-doc/TASK-FILE.md`：

| 字段 | 旧（references） | 新（agent 实际使用） |
|------|-----------------|---------------------|
| 任务标识 | `T1：<标题>` | `TASK-T001: <标题>` |
| 优先级 | `level: P0` | `priority: P0` |
| 描述 | `details：<描述>` | 移除（标题即描述） |
| 依赖 | `depends on：无` | `depends_on: []` |
| 完成标准 | `Done when：<标准>` | `done_when:\n    - <标准>` |
| 新增字段 | — | `refs`, `files`, `parallel`, `complexity` |

同理 ISSUE.md 对齐 `agents/test-agent.md` 的格式。

### SKILL.md 同步内容

需要补充：
- audit mode 说明
- VERSION 文件和自动 tag
- 新的 task/issue 格式字段
- `/mode` 命令中 audit 的说明
- 双重 Review、连续执行模式

### iterate.sh 归档修复

当前 iterate.sh 对 PRD.md/SPEC.md/TEST.md 使用 `cp`（复制到 archive），导致旧内容残留在 dev-doc/ 中污染新迭代。改为 `mv`（移动），新迭代从空白开始。

### validate.sh 对齐

当前 validate.sh 检查 `Done when` 和 `level`，需要改为检查 `done_when` 和 `priority`。同时新增对 `refs`、`files` 字段的存在性检查。

### 三处 SKILL.md 统一

```
skills/dev-flow/SKILL.md         → 主文件（Codex 格式）
.claude/skills/dev-flow/SKILL.md → Claude Code 格式
.agents/skills/dev-flow/SKILL.md → Codex/AGENTS 格式
```

差异仅在"运行约定"段落（Agent 调度方式不同），其余内容必须相同。方案：保持三份文件，但内容段落通过 validate.sh 校验一致性。

## Acceptance

- SPEC-AC-001: `references/dev-doc/TASK-FILE.md` 中的格式与 `agents/task-agent.md` 中的格式完全一致（同一个 schema）；`references/dev-doc/ISSUE.md` 同理
- SPEC-AC-002: agents/*.md 中不再有完整格式模板定义（替换为引用语句）；commands/*.md 的 agent prompt 模板中格式来源标注为 "读取 references/..." 
- SPEC-AC-003: 三处 SKILL.md 包含 audit mode、VERSION、当前所有命令列表
- SPEC-AC-004: `scripts/init/validate.sh` 校验 `priority`（非 `level`）、`done_when`（非 `Done when`）；`bash tests/test_validate.sh` 全量通过
- SPEC-AC-005: `references/dev-flow-spec.md` 反映当前实际流程（含 audit mode、VERSION、mode 定义）
- SPEC-AC-006: `iterate.sh` 对 PRD.md/SPEC.md/TEST.md 使用 `mv` 而非 `cp`；iterate 后 dev-doc/ 中不残留旧 SPEC/PRD/TEST

## Risks

- **subagent 无法读 references/**：commands/*.md 的调度模板已明确"读取并拼入"，不依赖 agent 自行读取
- **三份 SKILL.md 再次 drift**：通过 validate.sh 增加一致性检查（pre-commit 时校验）
- **validate.sh 字段变更导致旧项目报错**：validate 对两种格式都接受（兼容过渡期），但 agent 只产出新格式

## Test Plan

- `tests/test_validate.sh` 更新后全量通过
- `tests/test_skills_docs.sh` 增加 SKILL.md 三文件一致性校验
- 手动验证：agents/*.md 中 grep 不到完整格式模板（只有引用语句）
- `bash tests/test_all.sh` 全量回归通过

## Self Check

- [x] 目标清楚
- [x] 边界清楚
- [x] 验收可测
- [x] 与当前 mode（quick）匹配
