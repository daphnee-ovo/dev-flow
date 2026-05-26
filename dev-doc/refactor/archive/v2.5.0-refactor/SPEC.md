# 技术规范（SPEC）：轻量工程管理重构

## 1. 概述

本分支目标是重构 dev-flow 的工程管理能力。方向不是大而全，而是小而美：先想清楚再做，保持轻量、规范和约束，并确保流程文档与真实项目状态同步。

本轮先做深度审计和方案设计，不直接进入实现。

## 2. 核心原则

- 先想清楚再做：需求、边界、方案、验收标准清楚后再实现。
- 轻量：只保留推动交付的文档和命令。
- 规范：PRD、SPEC、TASK、TEST、issue、archive 有稳定结构。
- 约束：阶段、hooks、检查和任务闭环能阻止 agent 跳过关键步骤。
- 目标必要性：只引入服务当前目标的能力。
- 同步性：dev-doc 必须和代码、版本、任务、测试、迭代同步。
- 模式适配：MVP、quick、fast、full 使用不同门槛。

## 3. 工作范围

### Must Have

- 建立外部项目候选池和审计证据文档。
- 深入审计 Superpowers、Spec Kit、OpenSpec、GSD、Kiro、BMad、Task Master、OpenHands。
- 反向批判外部项目，避免照搬复杂度。
- 审计当前 dev-doc、commands、hooks、tests、task、SPEC、issue 闭环。
- 用本分支核心理念反查 dev-flow 自身，标记 keep / improve / add / prune。
- 设计轻量目标流程、最小追踪 ID、`/check` 与 `/status` 门禁。
- 设计 SPEC/TASK 生成机制、最小 `/devtest` 闭环、测试升级路线、迁移方案。
- 按审阅意见剪掉不必要复杂度：默认不引入 artifact-index、大型 controller、并发 waves、task model 字段、单独 Change Delta。

### Non-goals

- 本轮不直接改 controller 实现。
- 本轮不直接引入复杂 GUI 工作流。
- 本轮不强制所有模式采用 TDD。
- 本轮不把 dev-flow 变成大而全项目管理平台。
- 本轮不把调研结果扩展成新复杂系统。

## 4. 外部吸收规则

每个外部能力必须回答：

- 是否服务 dev-flow 的核心目标。
- 是否帮助 agent 先想清楚再做。
- 是否保持轻量。
- 是否增强规范。
- 是否形成有效约束。
- 是否保持 dev-doc 与真实项目同步。
- 是否适配不同开发模式。

不满足这些条件的能力不采用，或降复杂度后再采用。

## 5. 验收契约

- `dev-doc/refactor/task/task_2026-05-25_1.md` 是当前分支唯一 active task 文件。
- `bash scripts/commands/status.sh` 应显示当前阶段为 DEV，任务进度为 `31/31`，下一步进入 `/test`。
- `bash scripts/commands/check.sh` 不应因为根 `dev-doc/` 的旧任务污染当前分支统计。
- 31 个审计与设计任务的统一产物为 `dev-doc/refactor/research/engineering-management-audit.md`。

## 6. 风险与约束

- 风险：外部项目复杂度被照搬，破坏轻量目标。
  - 缓解：每条吸收建议必须经过目标必要性和模式适配检查。
- 风险：dev-doc 与真实项目状态再次漂移。
  - 缓解：优先设计 `/check`、`/status`、branch doc-root 和轻量追踪 ID。
- 风险：审计任务过大导致迟迟不落地。
  - 缓解：先修已暴露的脚本失败，再做模板升级。
