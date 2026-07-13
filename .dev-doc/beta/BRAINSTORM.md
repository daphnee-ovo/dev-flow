# 头脑风暴记录 — 统一测试流程与 ISSUE 闭环

**日期**：2026-07-12

## 背景与目的

当前 `dow test` 过度依赖脚本扩展名，Rust、Python、JavaScript/TypeScript、Go 等文件可能被错误地当作 shell 脚本执行。项目级全量测试、Task 关联测试、Task 关闭前验证和 ISSUE 记录之间也没有形成一致闭环。

本次统一：

- `dow test` 执行项目级全量测试。
- `dow test TASK-ID` 执行指定 Task 关联的测试。
- `dow task done TASK-ID` 关闭前自动执行 Task 测试。
- 通过语言和测试工具适配器执行测试。
- 区分测试失败和测试前置条件失败。
- 真实测试失败继续通过现有 ISSUE 机制记录。
- 删除独立 `devtest` 概念，并同步清理、更新相关文档。

## 关键决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 全量测试 CLI | `dow test` | 表示项目级完整测试 |
| Task 测试 CLI | `dow test TASK-ID` | 命令语义直接表达测试目标 |
| 文件参数 | 移除 | 指定文件测试使用语言自己的命令或 `test.ci` |
| Task 测试来源 | 扫描 active 和 `done_` Task 文件中的 `files.test` | 已关闭 Task 仍需被识别 |
| 路径基准 | `project_root` | 所有相对路径统一相对于项目根目录 |
| Task 关闭 | 先测试，成功后才标记 done/重命名 | 测试失败时不能关闭 Task |
| 测试配置 | `.dev-doc/test.ci` | 统一承载全量和 Task 测试的自定义命令 |
| `devtest` | 不再作为独立流程保留 | Task 测试由新 CLI 和 Task 关闭流程触发 |
| ISSUE 来源 | `source: test` | 全量和 Task 测试统一来源 |
| ISSUE 标题 | 全量：`Test fail:xxx`；Task：`Test TASK-ID fail:xxx` | 标题直接表达范围和 Task 关联 |
| ISSUE 文件字段 | `files_modify`、`files_create` 可选 | 保持现有 ISSUE 数据结构，schema 展示 |
| 自动 ISSUE 严重级别 | 默认 `P1` | 测试失败默认按开发阻断问题记录 |
| 自动 ISSUE location | 优先第一个失败文件位置；没有文件时使用测试命令 | 满足现有 location 字段 |
| 文档同步 | 删除 `devtest.md`，更新 README、命令文档、Agent 文档和生成副本 | CLI、流程和文档必须一致 |

## 设计方案

### 架构

测试流程分为四层：

1. 目标解析层：解析全量测试或 `TASK-ID` 测试目标，读取 `files.test`，并把路径解析到 `project_root`。
2. 测试计划层：根据项目描述文件、测试文件和 `.dev-doc/test.ci` 生成测试命令及前置条件。
3. 执行与分类层：执行所有计划命令，保留原始 stdout/stderr，区分 PASS、TEST_FAILED、PRECONDITION_FAILED。
4. 流程闭环层：真实测试失败批量调用 `dow issue create`；前置条件失败只返回提示，不创建 ISSUE；Task 关闭只在测试完全通过后继续。

`/test` 是 TEST 流程入口，负责阶段切换、上下文组装和流程调度；实际测试计划只由 `dow test` 执行。`/test` 不重复执行同一批测试，测试失败 ISSUE 由 `dow test` 创建。TEST agent 只做未被 CLI 覆盖的独立补充验证，不重复创建同一失败 ISSUE。

`test.ci` 的命令是最终测试命令，不再额外包一层脚本：

```yaml
devtest:
  run: ...

test:
  run: ...
```

支持的占位符：

- `{{project_root}}`
- `{{task_id}}`
- `{{task_file}}`
- `{{test_files}}`：展开成多个 shell-safe 参数

`test:` 只影响全量测试；`devtest:` 只影响 Task 测试。缺少对应配置段时使用内置适配器。

### 全量测试

全量测试只扫描项目根目录的项目描述文件及明确声明的 workspace member，不递归扫描任意嵌套项目，也不进入 `node_modules`、`target`、`legacy` 等目录。

内置默认行为：

| 项目类型 | 默认命令 | 前置条件 |
|---|---|---|
| Rust workspace/package | workspace 级 Cargo test | Cargo 和 manifest 存在 |
| Go module | `go test ./...` | Go 和 `go.mod` 存在 |
| Python | `python -m pytest` | Python、pytest 和项目测试配置存在 |
| JavaScript/TypeScript | package manager 的 test script | package manager、`package.json` 和 test script 存在 |

JavaScript/TypeScript 支持 npm、pnpm、yarn、bun。package manager 根据 `package.json` 的 `packageManager` 或唯一 lockfile 判断；无法确定时属于前置条件失败。没有 test script 时不猜测测试框架，提示通过 `test.ci` 配置。

全量测试默认只执行测试命令，不自动追加 check、build 或类型检查命令；需要时通过 `test.ci` 配置。

Shell 作为兼容适配器保留。全量测试扫描根目录 tests/test_*.sh，排除聚合脚本 test_all.sh；项目中存在其他语言测试时，Shell 测试也一并执行。

Python 全量测试要求存在 pytest.ini、带有 pytest 配置的 pyproject.toml，或带有 pytest 配置的 setup.cfg，并确认 python -m pytest 可用。Python Task 测试只要求关联文件存在且 pytest 可用，不额外要求项目配置文件。

### Task 测试

`dow test TASK-ID` 扫描 active 和 `done_` Task 文件，收集该 Task 的 `files.test`。测试文件按最近的项目描述文件和语言适配器分组执行。

当 Task 的 `files.test` 为空时，表示该 Task 没有 Task 级测试目标，结果为 PASS，不产生 PRECONDITION_FAILED。Task 仍需按照自身的 done_when 完成其他验收要求。

内置适配器：

| 语言/目标 | 默认行为 |
|---|---|
| Rust integration test | 在最近 crate 下执行对应 Cargo integration test |
| Rust inline test | 无法按源文件精确选择时，回退执行 crate/package 级 Cargo test，并明确报告范围扩大 |
| Python | 使用存在的 pytest 执行测试文件 |
| JavaScript/TypeScript | 使用已识别的本地 Vitest、Jest 或 Node test runner；未知 runner 进入前置条件失败 |
| Go | 按最近 `go.mod` 和 package 目录分组执行 package 级 Go test，不伪装成精确文件测试 |
| Shell | 保留现有直接执行兼容行为 |

不支持的语言、测试框架、缺失测试工具、缺失 manifest、缺失关联文件、无法确定 package manager 或无效 `test.ci`，都属于前置条件失败，不直接把文件交给 `bash`。

### 测试配置

配置命令直接作为最终命令执行。自定义命令优先于对应内置适配器；没有配置时才使用内置适配器。

`{{test_files}}` 必须按参数列表安全展开，避免把多个文件拼成一个参数，也避免路径中的空格导致命令解析错误。

每条 test.ci run 命令都以 project_root 为工作目录，继承当前进程环境变量，并作为独立 shell 命令执行。命令不存在、shell 无法启动或占位符无法解析时属于 PRECONDITION_FAILED；命令已经启动后返回非零才属于 TEST_FAILED。

### 状态与错误处理

执行结果分为：

- `PASS`：所有计划命令的前置条件满足，且全部返回 0。
- `TEST_FAILED`：测试工具已经启动，但测试断言或测试命令返回非 0。
- `PRECONDITION_FAILED`：测试没有正常启动，例如缺失工具、manifest、测试文件、runner 或配置错误。

所有计划命令都执行完成后再汇总结果：

- 只有 `TEST_FAILED`：退出码 1，创建 ISSUE。
- 只有 `PRECONDITION_FAILED`：退出码 2，不创建 ISSUE。
- 两者同时出现：退出码 1，只为真实测试失败创建 ISSUE；前置条件失败原样显示但不创建 ISSUE。
- 所有 stdout/stderr 原样保留在终端输出，并将完整失败消息写入 ISSUE 的 `desc`。

`dow test TASK-ID` 不改变 Task 状态。`dow task done TASK-ID` 使用事务式顺序：测试通过后才标记 `[x]` 并重命名；任何失败都保持原 Task 文件不变。

`dow task done` 支持多个 ID，但每个 ID 必须先完成自己的测试和状态写入；任一 ID 失败时，后续 ID 不再处理，已完成 ID 不回滚。单 ID 和多 ID 的文件写入、重命名失败都必须返回错误，并保持当前文件内容不被部分改写。

### ISSUE 创建

issue create 保持现有字段，并增加可选的 files_modify、files_create 输入。

测试 ISSUE 示例字段：title、severity、location、desc、reproduce、source、files_modify、files_create。

files_modify 和 files_create 没有传入时按空数组处理。issue schema 必须展示这两个字段，类型是字符串数组，且不是必填字段。

issue create 保持现有命令行单条创建能力，并支持单个 JSON 对象或 JSON 数组批量创建。保留现有 desc 参数兼容。fix 仍然只能通过 issue update 填写。

本次不新增 scope、refs 或 files_test 字段。

测试失败映射：

| ISSUE 字段 | 全量测试 | Task 测试 |
|---|---|---|
| title | Test fail:xxx | Test TASK-ID fail:xxx |
| severity | P1 | P1 |
| location | 第一个失败文件位置；无文件时使用测试命令 | 同左 |
| desc | 完整原始失败消息 | 完整原始失败消息 |
| reproduce | 实际执行命令和 project root | 实际执行命令和 project root |
| source | test | test |
| files_modify/files_create | 有明确范围时传入，否则省略 | 有明确范围时传入，否则省略 |

一次测试执行中多个命令失败时，构造 JSON 数组交给 issue create，由 ISSUE 批量创建逻辑写入同一个 ISSUE 文件中的多个条目。前置条件失败不进入该流程。

ISSUE 批量输入使用与单对象相同的字段契约：title、severity、location、desc、reproduce、source 必填，files_modify 和 files_create 可选，fix 在创建时拒绝。命令行 flags 继续只支持单条创建。

JSON 数组按输入顺序分配全局 ISSUE ID，并按 source 和日期分组写入 ISSUE 文件；每个分组的 nums 等于该文件中的条目数。所有对象先完成字段校验，再开始写入；校验失败不能产生部分 ISSUE。写入失败必须返回错误，并尽量保持已有文件不变。

## 约束与边界

本次不保留独立的 devtest 命令、devtest.md 流程或 source: devtest 自动 ISSUE 语义。devtest 只保留为 test.ci 中的配置段名称，用于 Task 级测试自定义。

不把 done_when 中的命令当作测试定义；测试关系只读取 files.test 或 test.ci。不递归发现未声明的嵌套项目，不自动猜测未知 JavaScript/TypeScript runner，也不把不支持的测试文件交给 shell。

前置条件失败不创建 ISSUE。Task ID 通过 ISSUE 标题关联。所有相对路径以 project_root 为基准。

文档更新必须覆盖实际分发入口和生成副本，避免不同 Agent 的说明分叉。

## 文档影响范围

文档同步是独立的 docs Task，不与测试实现 Task 混合。当前实现入口以 dow doctor 为准；旧的 check、validate、devtest 等文档引用需要在该独立 Task 中核对并更新。

dow doctor 是当前诊断、校验和修复入口；dow fix 只保留为兼容别名。dow check 和 dow validate 不再作为现行 CLI 入口。无修复参数的 dow doctor 仍可能创建缺失目录或更新 .gitignore，因此文档不能声明它绝对只读。

实现时需要检查并同步：

删除 plugin/commands/devtest.md，并清理对应生成的 skill/命令引用。

更新 README.md、README.zh-CN.md 和 npm/dev-flow/README.md 中的命令表、流程图、目录说明和测试说明。

更新 plugin/commands/test.md，删除独立 devtest 流程，说明 dow test 与 dow test TASK-ID。

更新 plugin/commands/issue.md，说明批量 JSON、可选 files_modify/files_create 和 issue schema。

更新 plugin/commands/task.md，说明 dow task done TASK-ID 的自动测试门禁。

更新 plugin/agents/test-agent.md 及相关 workflow 注入文档。

执行项目要求的 skill 同步流程，检查 Codex、Claude Code 和其他生成副本。

全量搜索 devtest、旧的文件选择参数和旧的 Task 测试语义，避免残留文档与实现冲突。

## 下一步

已跳过 PRD，进入 /spec，细化测试适配器、命令解析、ISSUE 批量写入和事务式 Task 关闭实现。

## 说明

本记录只描述已经确认的行为和边界，不代表代码已经实现。实现前仍需创建对应 Task，按 dev-flow 流程 claim；实现后需要验证 CLI、五种语言适配器、失败分类、ISSUE 批量创建、Task 关闭门禁和文档同步。
