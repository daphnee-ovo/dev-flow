# Task 文件格式规范

## 路径

`.dev-doc/<branch>/task/task_<YYYY-MM-DD>_<seq>.md`（使用 `dow task create` 创建）

完成标记：hook 自动重命名为 `done_task_<YYYY-MM-DD>_<seq>.md`

## Public Input

Task create/update accepts a nested file object via `--file`, for example
`--file '{"modify":["src/main.rs"],"test":["tests/task.rs"]}'`. Stdin JSON
places the same object under the top-level `files` key. `create` and `modify`
are individually optional, but at least one must contain a non-empty path;
`test` is optional.

## 模板

```markdown
---
title: TASK - <批次主题>
nums: <任务总数>
---

- [ ] TASK-T001: <任务名称>
  - type: feat
  - priority: P0
  - refs: SPEC-AC-001 或 user-request
  - files:
      create: []
      modify: ["path/to/file"]
      test: ["tests/test_x.sh"]
  - depends_on: []
  - parallel: false
  - complexity: S
  - done_when:
      - <客观可验证的验收标准>

- [ ] TASK-T002: <任务名称>
  - type: refactor
  - priority: P1
  - refs: SPEC-AC-002
  - files:
      create: ["src/new_module.ts"]
      modify: []
      test: ["tests/test_module.sh"]
  - depends_on: [TASK-T001]
  - parallel: true
  - complexity: M
  - done_when:
      - <验收标准 1>
      - <验收标准 2>

- [x] TASK-T003: <任务名称>（已完成）
  - type: test
  - priority: P0
  - refs: SPEC-AC-001
  - files:
      create: []
      modify: ["src/core.ts"]
      test: ["tests/test_core.sh"]
  - depends_on: []
  - parallel: false
  - complexity: S
  - done_when:
      - `bash tests/test_core.sh` 全部 PASS
```

## 字段说明

| 字段 | 说明 |
|------|------|
| title | yaml 头，批次主题 |
| nums | yaml 头，该文件中任务总数 |
| type | 任务类型（详见下方 Type 定义） |
| priority | P0=阻塞后续任务 / P1=重要不阻塞 / P2=可选优化 |
| refs | 关联的 SPEC 验收条件或需求来源 |
| files.create | 需要新建的文件列表 |
| files.modify | 需要修改的文件列表 |
| files.test | 对应的测试文件列表(如果存在则为对应文件，如果不存在则为应创建测试文件) |
| depends_on | 前置依赖的任务标识（可跨文件引用：`<文件名>:TASK-T00N`） |
| parallel | 是否可与同级任务并行执行 |
| complexity | S=小 / M=中 / L=大（详见下方定义） |
| done_when | 可验证的完成标准列表（必须客观具体） |

## Type 定义

| 值 | 含义 |
|------|------|
| `feat` | 新特性 |
| `fix` | 修复缺陷 |
| `refactor` | 代码重构 |
| `docs` | 文档修改 |
| `perf` | 优化代码，提高性能 |
| `test` | 测试用例修改 |
| `style` | 代码格式修改 |

## Priority 定义

- **P0**：阻塞后续任务或项目核心功能，必须最先完成
- **P1**：重要但不阻塞其他任务，P0 全部完成后执行
- **P2**：可选优化，所有 P0/P1 完成后有余力再做

判断标准：如果这个任务不做，其他任务能否继续？能 → P1/P2；不能 → P0。

## Complexity 定义

| 值 | 含义 | 判断标准 | 推荐工作模型 |
|------|------|----------|-------------|
| `S` | 小任务 | 影响 <=2 文件，有明确模板/规范可循 | 简单模型 |
| `M` | 中等任务 | 影响 3-5 文件或需要理解模块交互 | 正常或高级模型 |
| `L` | 大任务 | 涉及架构调整或 SPEC 中未明确的权衡，必须拆分或说明原因 | 高级模型 |

## done_when 规范

使用列表格式，每项为一个独立的验收标准。

**优先使用可执行格式**：`command | expected_output` 或 `command → exit_code`

优秀（可自动验证）：
- "`bash tests/test_auth.sh` 全部 PASS"
- "`curl -s /api/users | jq length` 输出大于 0"
- "`source calc.sh && divide 1 0 2>&1` 输出包含 error 且退出码非 0"

合格（可人工验证）：
- "运行 `npm test` 全部通过"
- "访问 /login 能看到表单，错误密码显示红色提示"

不合格（模糊、不可验证）：
- "完成"、"实现了"、"代码写好了"、"输出错误信息"

## 状态标记

- `- [ ]`：未完成
- `- [x]`：已完成

## 完成规则

- 完成任务后需手动将 `[ ]` 改为 `[x]`，但不需要手动重命名文件（hook 自动完成）
- 文件内所有 checkbox 均为 `[x]` → hook 自动重命名为 `done_` 前缀
- `/iterate` 时 `done_task_*.md` 自动归档到 `dev-doc/archive.db`（SQLite），源文件删除
- `/iterate` 前会阻断未完成 task；能进入归档时，active task 文件已经全部完成并会随 done_task 一起归档

## 命名规则

- `seq`：当天的序号，从 1 开始
- 创建新 task：`dow task create`（自动计算序号）
