# SPEC: 统一测试执行与 ISSUE 闭环

## Goal

将 `dow test` 改造成项目级和 Task 级统一测试入口，使用语言与测试工具适配器执行测试，区分测试失败和前置条件失败，并在真实测试失败时通过现有 ISSUE 领域逻辑记录结果。`dow task done TASK-ID` 必须在测试通过后才关闭 Task。

本 SPEC 基于 BRAINSTORM，当前 mode 为 fast，因此跳过 PRD，直接进入技术实现设计。

## Scope

### In Scope

- `dow test` 全量测试。
- `dow test TASK-ID` Task 级测试。
- Rust、Python、JavaScript、TypeScript、Go 和 Shell 兼容适配器。
- `.dev-doc/test.ci` 的全量和 Task 级自定义命令。
- 测试结果分类、退出码、原始输出保留和 ISSUE 创建。
- `dow issue create` 的可选 files 字段和 JSON 批量输入。
- `dow task done` 的测试门禁和失败保护。
- `/test` 与 `dow test` 的职责边界。

### Out of Scope

- 不保留独立的 `dow devtest` 流程。
- 不新增 ISSUE 的 `scope`、`refs` 或 `files_test` 字段。
- 不自动执行 build、check 或类型检查。
- 不递归发现未声明的嵌套项目。
- 不为未知 JavaScript/TypeScript runner 做猜测。
- 不把不支持的测试文件直接交给 shell。
- 文档同步不与测试实现混在同一个代码 Task；由 `TASK-T001` 单独完成。

## Requirements Trace

| Requirement | Source |
|---|---|
| Full test uses `dow test` | BRAINSTORM CLI decisions |
| Task test uses `dow test TASK-ID` and Task files `files.test` | BRAINSTORM Task test |
| Active and done Task files are scanned | BRAINSTORM Task test source |
| Paths are relative to `project_root` | BRAINSTORM path decision |
| Task close is gated by test | BRAINSTORM workflow closure |
| `test.ci` has `devtest` and `test` sections | BRAINSTORM configuration |
| Precondition failure creates no ISSUE | BRAINSTORM error handling |
| Test ISSUE uses existing fields and optional files fields | BRAINSTORM ISSUE design |
| `/test` delegates execution responsibility to `dow test` | BRAINSTORM flow boundary |
| Documentation is a separate Task | User request and TASK-T001 |

## Design

### Command Model

`TestArgs` 只保留可选的 Task ID：

- 无 Task ID：构造 FullTest 目标。
- 有 Task ID：构造 TaskTest 目标。
- 移除文件选择参数。

`/test` 负责 TEST 阶段入口、阶段上下文和流程调度；实际测试计划只由 `dow test` 执行。`/test` 不重复运行相同计划，测试失败 ISSUE 由 `dow test` 创建。TEST agent 只负责 CLI 未覆盖的独立补充验证。

`dow task done` 在调用测试 runner 时直接使用 TaskTest 目标，不通过 CLI 子进程回调 `dow task done`，避免循环调用。

### Core Test Types

测试 runner 内部使用以下概念：

- `TestTarget`: `Full` 或带 Task ID 的 `Task`。
- `TestPlan`: 待执行命令、工作目录、来源文件、适配器名称和前置条件。
- `TestExecution`: 命令、退出状态、stdout、stderr、启动状态和关联文件。
- `TestOutcome`: `PASS`、`TEST_FAILED` 或 `PRECONDITION_FAILED`。

适配器只负责生成 `TestPlan`，执行器负责统一执行和分类，ISSUE 服务负责记录失败。不要让适配器直接写 ISSUE 或修改 Task 文件。

### Target and Path Resolution

所有相对路径先基于 `project_root` 解析为绝对路径，命令显示时保留 project root 下的相对形式。

Task 目标扫描 active 和 `done_` Task 文件，读取目标 Task 的 `files.test`。空 `files.test` 表示没有 Task 级测试目标，直接返回 PASS，不创建 ISSUE，也不返回前置条件失败。

全量目标只读取项目根目录的项目描述文件及明确声明的 workspace member，不递归扫描任意嵌套项目；跳过 `node_modules`、`target` 和 `legacy` 等目录。

### `test.ci` Configuration

配置格式保持缩进段结构：

```yaml
devtest:
  run: ...

test:
  run: ...
```

`test` 只覆盖 FullTest，`devtest` 只覆盖 TaskTest。缺少对应段时使用内置适配器。

每一条 `run` 都是最终 shell 命令，不再包一层脚本。执行工作目录固定为 `project_root`，继承当前进程环境变量，每条命令独立执行。

占位符：

- `{{project_root}}`
- `{{task_id}}`
- `{{task_file}}`
- `{{test_files}}`

`{{test_files}}` 由路径参数列表安全展开，不能把多个路径拼成一个参数。占位符无法解析、配置语法无效、shell 无法启动或命令不存在属于 PRECONDITION_FAILED。

### Full Test Adapters

| Target | Plan | Precondition |
|---|---|---|
| Rust workspace/package | 执行 workspace 或 package 级 Cargo test | Cargo 和 manifest 存在 |
| Go module | 执行 `go test ./...` | Go 和 `go.mod` 存在 |
| Python | 执行 `python -m pytest` | pytest 配置文件存在且 pytest 可用 |
| JavaScript/TypeScript | 执行 package manager 的 test script | package manager、package.json 和 test script 存在 |
| Shell compatibility | 执行根目录 `tests/test_*.sh`，排除 `test_all.sh` | 文件可读且 shell 可启动 |

JavaScript/TypeScript 的 package manager 从 package.json 的 packageManager 字段或唯一 lockfile 判断，支持 npm、pnpm、yarn、bun。无法唯一判断时属于 PRECONDITION_FAILED。没有 test script 时不自动猜测 runner，提示配置 `test.ci`。

全量测试中发现多个语言或 Shell 测试目标时全部加入计划并执行，不因某一项失败而提前停止。

### Task Test Adapters

Task 文件按最近的项目描述文件和语言适配器分组：

| Target | Plan |
|---|---|
| Rust integration test | 在最近 crate 执行对应 integration test |
| Rust inline test | 无法按源文件选择时执行 crate/package 级 Cargo test，并报告范围扩大 |
| Python | pytest 执行关联文件；只要求文件存在和 pytest 可用 |
| JavaScript/TypeScript | 识别本地 Vitest、Jest 或 Node test runner，通过项目 package manager 执行 |
| Go | 按最近 go.mod 和 package 目录执行 package 级 Go test |
| Shell | 保留直接 shell 执行兼容行为 |

缺失测试文件、manifest、测试工具、已知 runner 或有效配置属于 PRECONDITION_FAILED。未知语言或未知 runner 不得降级为 Shell 执行。

### Execution and Classification

执行器先做所有可确定的前置条件检查，再执行计划中的全部命令。每条命令保留完整 stdout 和 stderr。

- PASS：所有计划命令前置条件满足且退出码为 0。
- TEST_FAILED：测试工具已启动，测试断言失败或命令退出非零。
- PRECONDITION_FAILED：测试没有正常启动。

退出码：

- 全部 PASS：0。
- 存在 TEST_FAILED：1。
- 只有 PRECONDITION_FAILED：2。
- 两者同时出现：1。

混合结果只为 TEST_FAILED 创建 ISSUE；PRECONDITION_FAILED 只显示原始原因。TaskTest 本身不修改 Task 状态。

### Task Close Gate

`dow task done TASK-ID` 的顺序：

1. 定位 pending Task。
2. 使用 TaskTest 目标执行测试。
3. 非 PASS 立即返回，Task 内容和文件名保持不变。
4. PASS 后更新 checkbox。
5. 文件内没有未完成 Task 时，使用临时文件完成内容写入，再执行 done 文件重命名。

多 ID 调用按输入顺序逐个处理。某个 ID 失败时停止后续 ID；已完成 ID 不回滚。内容写入或重命名失败必须返回错误，并尽力恢复失败前的文件状态，不允许留下半写入的 Task 内容。

空 `files.test` 的 Task 直接通过测试门禁，仍然必须满足 `done_when`。

### ISSUE Service

把 ISSUE 的 ID 分配、字段校验、文件分组、Markdown 渲染和写入抽为可复用的内部服务，CLI 和 test runner 共同调用，不直接由 test runner 写 Markdown。

单对象和数组对象使用相同字段：

- 必填：title、severity、location、desc、reproduce、source。
- 可选：files_modify、files_create。
- fix 在创建时拒绝。

保留命令行单条创建。stdin 支持单个 JSON 对象和 JSON 数组。数组按输入顺序分配全局 ISSUE ID，按 source 和日期分组写入 ISSUE 文件，每个文件的 nums 等于条目数量。所有对象完成校验后才开始写入；校验失败不得产生部分 ISSUE。写入阶段使用临时文件，写入失败返回错误并尽量保持既有文件不变。

测试失败 ISSUE：

| Field | FullTest | TaskTest |
|---|---|---|
| title | `Test fail:xxx` | `Test TASK-ID fail:xxx` |
| severity | `P1` | `P1` |
| location | 首个失败文件位置；没有文件时使用测试命令 | 同左 |
| desc | 完整 stdout/stderr | 完整 stdout/stderr |
| reproduce | 实际命令和 project root | 实际命令和 project root |
| source | `test` | `test` |
| files_modify/files_create | 有明确范围时传入，否则省略 | 有明确范围时传入，否则省略 |

### Documentation Boundary

文档更新属于独立 `TASK-T001`，不与测试实现合并。该 Task 负责删除独立 devtest 文档、更新 README 和命令文档、修正 dow doctor 入口说明、更新 ISSUE schema 说明，并执行 `bash devtools/assemble.sh all` 验证生成副本。

## Acceptance

- SPEC-AC-001: 无参数执行 `dow test` 时，按项目描述文件发现 Rust、Go、Python、JavaScript/TypeScript 和 Shell 测试，不把非 Shell 测试文件交给 Shell。
- SPEC-AC-002: 执行 `dow test TASK-ID` 时，只使用 active 或 `done_` Task 中该 Task 的 `files.test`，所有相对路径基于 project root。
- SPEC-AC-003: Rust integration test、Rust inline test、Python、JavaScript/TypeScript、Go 的适配行为与本 SPEC 一致；缺失工具或 runner 返回 PRECONDITION_FAILED。
- SPEC-AC-004: `test.ci` 的 `test` 和 `devtest` 命令在 project root、继承环境、独立执行，并正确展开所有占位符。
- SPEC-AC-005: 测试失败返回 1，前置条件失败返回 2，混合结果返回 1；前置条件失败不创建 ISSUE。
- SPEC-AC-006: 测试失败输出保留完整 stdout/stderr，并生成标题分别为 `Test fail:xxx` 或 `Test TASK-ID fail:xxx` 的 `source: test` ISSUE。
- SPEC-AC-007: `issue create` 支持可选 files_modify/files_create、单对象和 JSON 数组，schema 展示两个可选数组字段，批量 ID 和 nums 分配正确。
- SPEC-AC-008: `dow task done TASK-ID` 只有在 TaskTest PASS 后才更新 checkbox 或重命名；测试、写入或重命名失败不会留下半写入状态。
- SPEC-AC-009: 空 `files.test` 的 TaskTest 返回 PASS，不创建 ISSUE。
- SPEC-AC-010: `/test` 不重复执行 `dow test` 的同一计划，不重复创建同一测试失败 ISSUE。
- SPEC-AC-011: TASK-T001 完成后，README、命令文档、Agent 文档、Claude manifest 和生成副本不再保留过时的入口说明。

## Risks

- 不同语言的 workspace 结构可能无法唯一判断测试根目录；遇到歧义必须返回前置条件失败或使用 test.ci。
- JS/TS package manager 和 runner 组合较多；内置支持必须限制在明确可识别的本地工具，未知组合交给 test.ci。
- Shell 命令的字符串解析无法完整推断任意自定义命令；已知适配器先做工具检查，自定义命令使用 shell 启动结果和命令缺失特征分类。
- ISSUE 批量跨多个 source 文件时存在多文件提交风险；测试 runner 默认只使用 source: test，ISSUE 服务必须先完成全部校验并使用临时文件。
- Task 文件写入和重命名不是系统事务；实现必须保留原内容快照，在失败时执行恢复。

## Test Plan

- 在 `tmp/test_target_project` 建立最小 Rust、Python、JS/TS、Go 和 Shell fixture，验证全量发现和 Task 适配器。
- 使用缺失工具、缺失 manifest、缺失 runner、缺失测试文件、无效 test.ci 验证 PRECONDITION_FAILED 且无 ISSUE。
- 使用真实断言失败验证原始 stdout/stderr、退出码和 ISSUE 字段。
- 验证 Rust inline test 的 package 级回退提示。
- 验证 test.ci cwd、环境变量、占位符和多个 run 的独立执行。
- 验证 issue create 单对象、数组、可选 files 字段、ID、nums 和校验失败不写入。
- 验证单 ID 和多 ID Task close 的 PASS、失败、写入失败、重命名失败和恢复行为。
- 验证 `/test` 只调度 `dow test`，不重复运行或重复建 ISSUE。
- 文档 Task 使用 `dow doctor -H` 和 `bash devtools/assemble.sh all` 验证文档与生成副本。

## Self Check

- [x] Goal is clear
- [x] Scope and non-goals are explicit
- [x] Requirements trace to BRAINSTORM and TASK-T001
- [x] Acceptance criteria are testable
- [x] Failure paths and preconditions are specified
- [x] Matches fast mode with PRD skipped
