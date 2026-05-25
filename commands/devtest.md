---
description: 开发中例行测试 — 任务级最小闭环
allowed-tools: Agent, Bash, Read, Write, Edit
---

# DEV-TEST — 例行测试

devtest 只做轻量闭环，不做大型 controller。

## 执行方式

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/devtest.sh"
```

模式切换：

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/devtest.sh" --continuous
bash "${CLAUDE_PLUGIN_ROOT}/scripts/commands/devtest.sh" --step
```

## 前置检查

1. 当前阶段必须是 `DEV`。
2. `task/` 中必须存在已勾选 `[x]` 的任务。
3. 每次任务完成后必须执行 devtest。

## 三种结果

| 结果 | 行为 |
|------|------|
| `PASS` | 保持 task 勾选；如果所有任务完成，提示 `/test` |
| `FAIL` | 取消当前 task 勾选，写入 `issue_devtest_<date>_<seq>.md`，停止推进 |
| `NEEDS_CONTEXT` | 保持 task 勾选，不继续推进，要求补充信息 |

## Agent 验证要求

如果需要独立 agent 验证，只传入最小上下文：

| 允许传入 | 禁止传入 |
|----------|----------|
| 当前 task 标题、refs、files、done_when | 其他无关 task |
| SPEC 中相关验收条目 | PRD.md |
| 项目上下文 `scripts/lib/context.sh` | 开发阶段对话历史 |

验证重点：

- 对照 `done_when` 和相关 SPEC 验收判断是否通过。
- 需要测试时，优先运行 task `files.test` 中列出的测试。
- 不引入默认 TDD，不扩展需求范围。
- 禁止使用系统临时目录，临时文件只能放项目 `temp/` 下。

## 输出协议

subagent 或主 agent 输出末尾必须给出：

```text
STATUS: PASS | FAIL | NEEDS_CONTEXT
DETAIL: <说明>
```

如果没有明确状态，按 `NEEDS_CONTEXT` 处理，不自动推进。
