---
title: Agent 真实测试发现的 dev-flow 缺陷
nums: 5
---

- [x] I1：depends on 无硬约束守卫
  - severity: P1
  - location: scripts/hooks/post-write.sh, dev-doc/task/
  - description: task 文件中的 depends on 字段仅为声明性文档，无守卫脚本阻止乱序执行。agent 可以在 T1 未完成时直接标记 T2 完成。需要在 post-write hook 的 task 完成检测逻辑中添加依赖检查：标记 Tn 为 [x] 前验证其所有 depends on 任务均已 [x]
  - fix: post-write.sh 添加依赖守卫逻辑，检测同文件内已完成任务的 depends on 是否满足

- [x] I2：Done when 标准缺乏可执行格式规范
  - severity: P1
  - location: agents/task-agent.md, commands/task.md
  - description: Done when 允许自然语言描述（如"输出错误信息"），导致验证标准有歧义。不同 agent/开发者对"通过"的理解可能不一致。建议 task-agent 在生成 Done when 时推荐使用可执行格式：`command | expected_output`（如 `divide 1 0 2>&1 | error: division by zero`），并在 validate.sh 中警告纯自然语言的 Done when
  - fix: task-agent.md 添加 Done when 规范章节，分优秀/合格/不合格三级，推荐可执行格式

- [x] I3：fast 模式下各优先级任务处理规则未明确
  - severity: P1
  - location: commands/mode.md
  - description: mode.md 未明确说明 fast 模式下 P1/P2 任务是否可推迟或跳过。agent 在 fast 模式中不确定是否必须完成所有优先级的任务才能 iterate。需在 mode.md 中明确：所有模式下 iterate 前必须完成全部 task（不分优先级），或明确声明 fast 模式下 P2 可推迟
  - fix: mode.md 为 full 和 fast 模式添加明确约束说明，P0/P1 必须完成，P2 可标记推迟但不可删除

- [x] I4：task 完成标记无时序校验
  - severity: P2
  - location: scripts/hooks/post-write.sh
  - description: agent 可以一次性将所有条目从 [ ] 批量改为 [x]，无法区分逐步完成与批量伪造。当前 post-write hook 不检查一次 write 中标记了多少条目。可考虑：（1）检测单次修改完成超过 N 个条目时输出警告；（2）配合 CHANGELOG 时间戳交叉验证
  - fix: post-write.sh 添加 git diff 检测，单次标记超过 2 个完成时输出警告

- [x] I5：hooks 依赖 Claude Code 运行环境，独立 agent 不受约束
  - severity: P2
  - location: hooks.json, scripts/hooks/
  - description: dev-flow 的所有防护机制（inject-context 阻断、post-write 检查、save-changelog）均通过 Claude Code hooks 触发。当 agent 在独立环境（如 Codex spawn_agent、/tmp 测试目录）运行时，hooks 不生效，所有约束失效。建议：（1）iterate.sh 等关键脚本内置完整性检查（已实现）；（2）在 agent 调度模板中强制要求 source hooks 或执行等效检查
  - fix: 创建 scripts/lib/guard.sh 独立守卫脚本，提供 guard_check_all 等函数，非 Claude Code 环境可直接 source 调用
