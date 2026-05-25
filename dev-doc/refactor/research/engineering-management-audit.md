# dev-flow 工程管理审计与剪枝方案

更新时间：2026-05-26
分支：`refactor`
文档根：`dev-doc/refactor`

本文件是 T1-T31 的统一产物。上一版把可学习项目中的很多能力都展开成了设计项，读起来过重，也容易把 dev-flow 推向“大而全”。本版按用户审阅意见重新裁剪：只保留对 dev-flow 目标必要的能力，避免为了追踪、controller、并发、模板完整性引入新复杂系统。

## 核心判断

dev-flow 需要的是小而美的工程管理约束：

- 先想清楚再做：实现前讲清目标、边界、方案、验收。
- 轻量：默认路径少文档、少字段、少概念。
- 规范：现有 `PRD / SPEC / task / issue / TEST / archive` 保持结构稳定。
- 约束：关键错误要能阻断，例如无任务开发、阶段错误、文档与项目脱节。
- 同步：dev-doc 必须跟 branch、任务、测试、版本、迭代同步。
- 模式适配：mvp/fast 允许快速验证，full/quick 提高规范门槛。

本轮调整后的方向：

- 保留：branch-specific `dev-doc/<branch>`、mode、hooks、`/check`、`/status`、`/iterate`。
- 加强：task 可执行性、SPEC 验收清晰度、issue 闭环、脚本真实阻断、macOS 兼容性。
- 降级：artifact index、复杂 controller、并发 waves、重型追踪矩阵、默认 TDD。
- 删除：外部持续发现机制作为产品能力、额外 docs 字段、task model 字段、单独 Change Delta 章节、verification 字段。

## 外部项目对标结论

| 项目 | 值得学 | 不照搬 | dev-flow 采用方式 |
| --- | --- | --- | --- |
| Superpowers | 先澄清、写计划、完成前验证 | 默认 TDD 和多轮审批太重 | 学“先想清楚”和“计划可执行”，不强制 TDD |
| GitHub Spec Kit | spec / plan / tasks 的一致性检查 | feature directory 和模板链太重 | 学 checklist 和 task file path，不引入多层目录 |
| OpenSpec | change proposal、archive、渐进严格度 | 单独 delta 体系会增加概念 | change 直接写入 SPEC notes 和 changelog |
| GSD | 持久状态、phase、drift gate | agent roster、配置面、并发执行太大 | 学状态同步和漂移检查，不复制框架 |
| Kiro | requirements / design / tasks、Quick Plan、hooks | GUI 面板和 Run all Tasks 无法直接迁移 | 转成轻量 `/status`、`/check`、task 依赖提示 |
| BMad | next guidance、story lifecycle、测试分层 | workflow/agent 术语太多 | 学“下一步导航”和“按模式分层测试” |
| Task Master | dependency-aware next task、复杂度 | MCP 工具体系和 JSON store 太重 | 只保留 `depends_on`、`complexity`、next-task 计算 |
| OpenHands | issue triage、环境/测试入口清晰 | 大型 CI/benchmark 不适合默认流程 | 学 issue 字段和标准验证入口记录 |

结论：外部项目是参考，不是蓝图。任何吸收都必须过这 5 个问题：

1. 是否让 agent 更容易先想清楚再做？
2. 是否能约束真实工程行为，而不只是多写文档？
3. 是否保持默认路径轻量？
4. 是否能和 branch、任务、测试、版本同步？
5. 是否能按 full/quick/fast/mvp 降级？

## 当前 dev-flow 问题

| 问题 | 证据 | 影响 | 修正方向 |
| --- | --- | --- | --- |
| task 字段不够可执行 | `dev-doc/refactor/task/task_2026-05-25_1.md` 仍以 details 为主 | agent 执行时容易反复猜路径和验收 | task 模板增加 refs/files/depends_on/complexity/done_when |
| SPEC 验收容易漂移 | `dev-doc/refactor/SPEC.md` 曾写固定阶段状态 | 文档可能很快与真实状态脱节 | SPEC 只写稳定契约，实时状态交给 STATUS/check |
| branch doc-root 还未成为全局规则 | skill/README 仍主要描述根 `dev-doc/` | agent 可能写错目录 | 所有命令统一优先 `dev-doc/<branch>` |
| `/check` 仍偏提醒 | `scripts/commands/check.sh` | 关键错误不能阻断 | 增加 ERROR/WARN/OK 分级 |
| `/status` 仍偏 phase 映射 | `scripts/commands/status.sh` | 下一步可能不符合真实状态 | 根据 task/issue/test/check 事实给下一步 |
| hook 有 macOS 兼容问题 | `tests/test_all.sh` 中 `sed`、`grep -P` 失败 | 流程约束在本机不可靠 | 先修 BSD sed/grep 兼容 |
| issue 闭环字段不足 | `commands/issue.md` | fix 缺少复现、原因、关闭条件 | issue 模板增加最少闭环字段 |
| 测试有关键词假阳性 | 多个 test 检查文档关键词 | 文档写了不代表流程生效 | 核心流程改用 fixture 行为测试 |

## 保留、改进、增加、剪掉

| 类型 | 内容 | 原因 |
| --- | --- | --- |
| keep | `dev-doc/` 生命周期文档 | 是 dev-flow 的核心事实源 |
| keep | `/mode` | 解决不同项目阶段的流程轻重 |
| keep | `/iterate` | 保证流程管理和真实版本同步 |
| keep | task 完成后触发 `/devtest` | 轻量但有效的开发内循环 |
| improve | branch-specific doc-root | 必须从部分支持变成所有命令一致 |
| improve | `/check` | 从提醒变成可作为门禁的健康检查 |
| improve | `/status` | 从静态阶段改为事实驱动下一步 |
| improve | SPEC 模板 | 保持短，但必须有 scope、AC、risk、test plan |
| improve | task 模板 | 增加必要执行字段，删除多余字段 |
| improve | issue 模板 | 让 `/fix` 有复现、原因、验收依据 |
| add | 轻量 trace ID | 只用 markdown 内 ID，不新增复杂索引文件 |
| add | macOS shell 兼容规则 | 当前测试已证明必须做 |
| prune | artifact-index 默认方案 | 额外状态源会增加同步负担 |
| prune | devtest 大型 controller 状态机 | 先做简单 PASS/FAIL/NEEDS_CONTEXT 解析 |
| prune | 并发 waves | 当前目标不必要，先只做 next task |
| prune | 默认 TDD | 对 mvp/fast 太重 |
| prune | task `model` 字段 | `complexity` 已足够暗示执行难度 |
| prune | task `verification` 字段 | `/devtest` 和 TEST 报告负责验证事实 |
| prune | SPEC 单独 `Change Delta` | 变更写到 trace notes 和 changelog 即可 |

## 轻量 SPEC 模型

SPEC 不应该变成大模板。默认结构：

```markdown
# SPEC: <主题>

## Goal
<目标>

## Scope
### In
### Out

## Requirements Trace
| Req | AC | Notes |
| --- | --- | --- |
| PRD-FR-001 或 user-request | SPEC-AC-001 | ADDED / MODIFIED / REMOVED 可写在这里 |

## Design
<必要方案。能短就短。>

## Acceptance
- SPEC-AC-001: <可测验收>

## Risks
- <风险和回退>

## Test Plan
- <最小验证方式>

## Self Check
- [ ] 目标清楚
- [ ] 边界清楚
- [ ] 验收可测
- [ ] 与当前 mode 匹配
```

按模式降级：

| Mode | 必填 |
| --- | --- |
| full | Goal、Scope、Trace、Design、Acceptance、Risks、Test Plan、Self Check |
| quick | Goal、Scope、Design、Acceptance、Test Plan |
| fast | Goal、Acceptance、Test Plan |
| mvp | Goal、Out of scope、Smoke Test |

不新增单独 `Change Delta`。变更是 SPEC 的 notes，不是另一套流程。

## 轻量 task 模型

task 是执行计划，不是又一份设计文档。默认模板：

```markdown
- [ ] TASK-T001: <标题>
  - priority: P0|P1|P2
  - refs: SPEC-AC-001 或 user-request
  - files:
      create: []
      modify: ["path/to/file"]
      test: ["tests/test_x.sh"]
  - depends_on: []
  - parallel: true|false
  - complexity: S|M|L
  - done_when:
      - <可测验收，包含必要 expected output>
```

裁剪说明：

- 不要 `model`。`complexity` 已足够表达任务难度。
- 不要 `steps`。task 本身就是执行框架，不需要硬拆步骤。
- 不要 `verification`。完成后 `/devtest` 自动触发验证，TEST 报告记录事实。
- 不要 `docs` 字段。文档引用放 `refs`，文件变更只记录真正要创建/修改/测试的路径。
- `expected_output` 放进 `done_when`，不要另起字段。

## 轻量 issue 模型

issue 只补闭环所需字段：

```markdown
- [ ] ISSUE-I001: <标题>
  - severity: P0|P1|P2
  - source: devtest|test|manual
  - refs: TASK-T001, SPEC-AC-001
  - location: path:line
  - current:
  - expected:
  - reproduce:
  - root_cause:
  - fix:
  - close_when:
```

规则：

- P0 阻断 `/iterate`。
- P1 在 full/quick 阻断发布，在 fast/mvp 可由用户确认后延期。
- P2 不阻断，但 `/status` 必须展示。
- `/fix` 必须先补 root_cause，再改代码。

## `/check` 和 `/status`

`/check` 只做必要健康检查，分三类输出：

| Rule | Level | 判断 |
| --- | --- | --- |
| DOCROOT | ERROR | 当前 branch 应优先使用 `dev-doc/<branch>`，不能被根 `dev-doc/` 污染 |
| TASK_NUMS | ERROR | `nums` 与 checkbox 数不一致 |
| TASK_DEPS | ERROR | 已完成 task 依赖未完成 |
| SPEC_AC | ERROR/WARN | 当前 mode 必填验收缺失 |
| OPEN_ISSUE | ERROR/WARN | P0/P1 未关闭 |
| TEST_REPORT | WARN | task 全完成但 TEST 缺失或过期 |
| VERSION_SYNC | WARN | VERSION 与 git tag 不同步 |
| SHELL_COMPAT | ERROR | 脚本使用 macOS 不兼容的 `sed -i`、`grep -P` 等核心路径 |

`/status` 不再只按 phase 给下一步：

1. 有 ERROR：提示先 `/check` 或具体修复项。
2. 有 P0/P1 issue：提示 `/fix`。
3. 有可执行 task：显示 next task。
4. task 全完成但 TEST 不完整：提示 `/test`。
5. TEST 通过且 release gate 满足：提示 `/iterate`。

## `/devtest` 最小闭环

先不做大型 controller。只实现三个结果：

| 结果 | 行为 |
| --- | --- |
| PASS | 保持 task 勾选，继续下一个任务或提示 `/test` |
| FAIL | 取消 task 勾选，写 issue，停止推进 |
| NEEDS_CONTEXT | 不取消 task，不继续推进，向用户要信息 |

复杂的 `DONE_WITH_CONCERNS`、多轮 review、并发推进可以以后再说。当前目标是让 task 完成后有真实测试反馈和 issue 闭环。

## 测试路线

先修当前已经暴露的真实失败，再谈新能力：

1. 修 `mode.sh` malformed phase。
2. 修所有 BSD `sed` 兼容问题。
3. 替换核心脚本中的 `grep -P`。
4. 修 `migrate.sh` phase 更新。
5. 修 `scan-project.sh` task/issue 统计。
6. 修 `block-non-dev-edit.sh` 阶段守卫。
7. 为 `/check`、`/status`、task parser、issue parser 增加 fixture 行为测试。
8. 减少只检查文档关键词的测试，把核心正确性转成输入/输出断言。

已知全量测试失败记录在 `dev-doc/refactor/TEST.md`。

## 迁移和兼容

原则：

- 不重写历史 archive。
- 旧 dev-doc 不强制补全新版字段。
- 不新增默认 `artifact-index.yaml`，避免多一个同步源。
- 遇到旧 dev-doc，直接在下一次 `/iterate` 时归档保存。
- 未关闭 task、issue 迁移到新一轮，并按新模板补齐必要字段。
- 脚本解析失败时先 WARN，不要静默写错目录。

如以后确实需要索引，只能作为 `/check --repair` 生成的缓存，不作为用户必须维护的文档。

## 分阶段实施

| 阶段 | 目标 | 文件 | 验证 |
| --- | --- | --- | --- |
| P0-1 | 修测试已暴露的脚本兼容问题 | `scripts/**/*.sh` | `bash tests/test_all.sh` 至少消除 sed/grep/mode/migrate/scan 容错类失败 |
| P0-2 | 统一 branch doc-root | `scripts/lib`、commands、hooks | branch fixture：不被根 `dev-doc/` 污染 |
| P0-3 | `/check` 增加 ERROR/WARN/OK | `scripts/commands/check.sh` | bad nums / open P0 / missing TEST fixture |
| P0-4 | `/status` 事实驱动 next | `scripts/commands/status.sh` | open issue、next task、all done 三类输出 |
| P1-1 | task 模板轻量升级 | `commands/task.md`、task agent | 新旧 task 都能解析 |
| P1-2 | SPEC 模板轻量升级 | `commands/spec.md`、spec agent | full/quick/fast/mvp 必填项不同 |
| P1-3 | issue 模板闭环 | `commands/issue.md`、`commands/fix.md` | FAIL 写 issue，fix 后 close_when 可验证 |
| P1-4 | `/devtest` 最小三状态 | `commands/devtest.md`、脚本 | PASS/FAIL/NEEDS_CONTEXT fixture |
| P2 | 只读依赖图和并行提示 | `/status` | 不自动并发写文件 |

## Adopt / Reject 决策

| ID | 决策 | 结果 |
| --- | --- | --- |
| ADR-001 | branch-specific dev-doc 是当前分支事实源 | Adopt |
| ADR-002 | task 增加 refs/files/deps/complexity/done_when | Adopt |
| ADR-003 | task 不增加 model/steps/verification/docs 字段 | Reject |
| ADR-004 | SPEC 不增加单独 Change Delta 章节 | Reject |
| ADR-005 | 不默认引入 artifact-index | Reject |
| ADR-006 | `/check` 从提醒升级为轻量门禁 | Adopt |
| ADR-007 | `/status` 从 phase mapping 升级为事实驱动 | Adopt |
| ADR-008 | `/devtest` 先做三状态，不做大型 controller | Adapt |
| ADR-009 | 不做默认 TDD | Reject |
| ADR-010 | 不做默认并发 waves | Reject |

## 31 任务完成索引

| Task | 本文件覆盖 |
| --- | --- |
| T1 | 外部候选池与筛选标准 |
| T2 | Evidence / Finding / Adopt / Reject / Task Mapping |
| T3 | Superpowers 审计 |
| T4 | Spec Kit 审计 |
| T5 | OpenSpec 审计 |
| T6 | GSD 审计 |
| T7 | Kiro 审计 |
| T8 | BMad 审计 |
| T9 | Task Master 审计 |
| T10 | OpenHands 审计 |
| T11 | 外部发现机制已降级为审计习惯，不作为产品能力 |
| T12 | 外部项目反向批判 |
| T13 | 评估框架已简化为问题表和改进方向 |
| T14 | artifact 一致性审计已保留关键问题 |
| T15 | keep / improve / add / prune |
| T16 | 命令流程问题已合并到 `/check` `/status` |
| T17 | hooks 问题已合并到脚本兼容和阻断规则 |
| T18 | 测试体系审计和升级路线 |
| T19 | 轻量 task 模型 |
| T20 | 轻量 SPEC 模型 |
| T21 | issue 闭环 |
| T22 | 目标流程已简化为必要闭环 |
| T23 | trace ID 保留，artifact-index 默认拒绝 |
| T24 | `/check` 与 `/status` 健康规则 |
| T25 | SPEC self-check |
| T26 | TASK 生成机制 |
| T27 | `/devtest` 最小三状态 |
| T28 | 并发降级为只读依赖提示 |
| T29 | 测试升级路线 |
| T30 | 迁移兼容方案 |
| T31 | 分阶段实施路线图 |
