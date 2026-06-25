# SPEC: dow CLI 重构 — 沉默原则 + 最小惊讶 + 资源模型

## Goal

重构 dow CLI 的命令接口层：统一输出行为（沉默原则）、重组命令体系（资源模型 noun+verb）、将 `.dev-doc/` 结构型文件的全部操作收归 dow 管理。

## Acceptance

- SPEC-AC-001: 动作型命令成功且无额外信息时 exit 0 + stdout 为空
- SPEC-AC-002: `dow task create/list/show/done/reopen/schema` 完整可用
- SPEC-AC-003: `dow issue create/list/show/close/reopen/schema` 完整可用
- SPEC-AC-004: `dow lint [--fix]` 替代原 check + validate + fix
- SPEC-AC-005: `dow test --task <ID>` 替代原 devtest
- SPEC-AC-006: `dow status set` 子命令替代写入 flags
- SPEC-AC-007: `dow doc` 命令消除，功能拆入各资源命令
- SPEC-AC-008: `dow prd create/schema`、`dow spec create/schema`、`dow brainstorm create/schema`、`dow changelog list/add/schema` 可用
- SPEC-AC-009: guard hook 正确区分文档型（放行编辑）和结构型（拦截编辑）
- SPEC-AC-010: iterate/task reopen/issue reopen 的 confirm 机制使用 token 前缀区分（ITR-/TRO-/IRO-），confirm 时重传参数
- SPEC-AC-011: 注入指令（`dow hooks context`）更新，反映新命令体系

## Design

### 1. 命令体系总览

```
dow
├── task        create | list | show <ID> | done <ID> | reopen <ID> [--confirm TRO-xxx] | schema
├── issue       create | list | show <ID> | close <ID> | reopen <ID> [--confirm IRO-xxx] | schema
├── changelog   list | add | schema
├── brainstorm  create | schema
├── prd         create | schema
├── spec        create | schema
├── init        --name <n> --mode <m>
├── status      (读) | set --phase/--mode/--exec-mode/--name/--goals-minor/--goals-major
├── iterate     --topic <t> --type <T> [--files ...] [-v patch] [--confirm ITR-xxx]
├── version     [--bump patch|minor|major] [--set X.Y.Z]
├── test        [--task <ID>] [--file <f>]
├── lint        [--fix]
├── scan
├── claim       [IDs] [--revoke]
├── rollback    [--list | --version <v>]
├── archive     (子命令不变)
├── hooks       context | guard | post-write | post-bash | save-changelog
└── setup       [--agent claude|codex|all]
```

### 2. 模块拆分计划

#### 2.1 新增命令文件

| 新文件 | 职责 | 来源 |
|--------|------|------|
| `commands/task.rs` | task 资源全部操作 | 从 `doc.rs`（create_task）+ `task_store.rs` + 新增 list/show/done/reopen |
| `commands/changelog_cmd.rs` | changelog list/add/schema | 从 `doc.rs`（changelog 部分）+ iterate.rs（read_changelog） |
| `commands/brainstorm.rs` | brainstorm create/schema | 从 `doc.rs` |
| `commands/prd.rs` | prd create/schema | 从 `doc.rs` |
| `commands/spec_cmd.rs` | spec create/schema | 从 `doc.rs` |
| `commands/lint.rs` | 合并 check + validate + fix | 合并三个现有文件 |

#### 2.2 修改现有文件

| 文件 | 变更 |
|------|------|
| `cli.rs` | 重写 Commands enum：新增 Task/Issue/Changelog/Brainstorm/Prd/Spec/Lint 子命令，移除 Doc/Check/Validate/Fix/Devtest |
| `commands/issue.rs` | 扩展：新增 create/show/close/reopen/schema 子命令 |
| `commands/status.rs` | 拆分读写：无 set 参数=读取，`set` 子命令=写入 |
| `commands/test_runner.rs` | 合并 devtest 逻辑，新增 `--task` flag |
| `commands/iterate.rs` | token 前缀改为 ITR-，confirm 时验证参数匹配 |
| `hooks/guard.rs` | 更新 `check_devdoc_direct_create`：文档型已存在=放行编辑，结构型永远拦截 |
| `hooks/context.rs` | 更新注入内容：反映新命令体系 |
| `main.rs` | 更新 dispatch match |

#### 2.3 删除文件

| 文件 | 原因 |
|------|------|
| `commands/doc.rs` | 职责拆散到 task/issue/prd/spec/brainstorm/changelog |
| `commands/check.rs` | 合并入 lint.rs |
| `commands/validate.rs` | 合并入 lint.rs |
| `commands/fix.rs` | 合并入 lint.rs |
| `commands/devtest.rs` | 合并入 test_runner.rs |

### 3. task 资源接口设计

#### 3.1 数据模型

```rust
struct TaskItem {
    id: String,           // "TASK-T001"
    title: String,
    task_type: String,    // feat/fix/refactor/docs/perf/test/style
    priority: String,     // P0/P1/P2
    refs: String,         // "SPEC-AC-001" 或 "user-request"
    files: TaskFiles,
    depends_on: Vec<String>,
    parallel: bool,
    complexity: String,   // S/M/L/XL
    done_when: Vec<String>,
    status: TaskStatus,   // Pending/Done
}

struct TaskFiles {
    create: Vec<String>,
    modify: Vec<String>,
    test: Vec<String>,
}

enum TaskStatus { Pending, Done }
```

#### 3.2 命令签名

```bash
# create：flags 或 stdin JSON
dow task create --title "..." --type feat --priority P0 \
    --refs "SPEC-AC-001" --files-modify "a.rs,b.rs" \
    --done-when "test passes"
# 或
echo '{"title":"...", ...}' | dow task create
echo '[{...},{...}]' | dow task create  # 数组=批量

# list
dow task list                     # 默认只展示 pending
dow task list --all               # 包含 done

# show
dow task show T001                # 单条详情（JSON）

# done
dow task done T001                # 标记完成（静默）

# reopen
dow task reopen T001              # 输出影响 + token
dow task reopen T001 --confirm TRO-xxxxxx  # 执行

# schema
dow task schema                   # 输出字段定义 JSON
```

#### 3.3 文件层存储

- 按批次文件：`task/task_YYYY-MM-DD_N.md`
- 单条 create 也写入当日批次文件（追加或创建）
- done 操作：在文件中把 `- [ ]` 改为 `- [x]`，文件所有条目都 done 后文件名加 `done_` 前缀
- reopen：把 `- [x]` 改回 `- [ ]`，如文件有 `done_` 前缀则去掉

### 4. issue 资源接口设计

```bash
dow issue create --title "..." --severity P0 --location "..." --desc "..."
# 或
echo '{"title":"...", ...}' | dow issue create

dow issue list                    # 默认只展示 open
dow issue list --all              # 包含 closed
dow issue show I001
dow issue close I001              # 静默
dow issue reopen I001             # 输出影响 + token
dow issue reopen I001 --confirm IRO-xxxxxx
dow issue schema
```

存储：保持一条一个文件（`issue/issue_<source>_<date>_<seq>.md`），close = 文件名加 `closed_` 前缀。

### 5. 文档型命令设计

```bash
# prd
dow prd create                    # 创建 PRD.md（按 mode 生成骨架）
dow prd schema                    # 输出格式定义

# spec
dow spec create
dow spec schema

# brainstorm
dow brainstorm create             # 创建 BRAINSTORM.md
dow brainstorm schema

# changelog
dow changelog list                # 输出当前 CHANGELOG.md 条目
dow changelog add --text "..."    # 追加一条
dow changelog schema
```

### 6. status 读写分离

```bash
# 读取（现有行为不变）
dow status                        # 全部字段
dow status --field phase          # 单字段

# 写入（新 set 子命令）
dow status set --phase DEV
dow status set --mode fast
dow status set --goals-minor "..."
```

CLI 定义：

```rust
#[derive(Subcommand)]
enum StatusCommands {
    Set(StatusSetArgs),
}

// dow status [--field F]  → 读取（StatusArgs 保持 field only）
// dow status set ...      → 写入
```

### 7. lint 命令（合并 check + validate + fix）

```bash
dow lint                          # 运行全部检查（结构 + 规范 + 一致性）
dow lint --fix                    # 自动修复可修复项
```

合并逻辑：
1. validate.rs 的目录结构检查
2. check.rs 的文档同步检查（changelog、task 完成度、issue 状态、时间同步、phase 文件）
3. doc_validator.rs 的格式校验
4. fix.rs 的自动修复（仅在 --fix 时执行）

输出格式保持：`{pass, errors, warnings, ok, fixed?}`

### 8. 沉默原则实现

在 `output.rs` 中统一处理：

```rust
/// 动作型命令结果：成功且无额外信息时什么都不输出
pub fn action_result(info: Option<&str>, human: bool) {
    if let Some(msg) = info {
        if human {
            println!("{}", msg);
        } else {
            // JSON 模式下有额外信息才输出
            print_json(&serde_json::json!({"info": msg}));
        }
    }
    // None = 完全静默
}
```

各命令适配：
- `task done`/`issue close`/`claim`/`claim --revoke`/`init`（无 .dev-doc 外副作用）/`lint`（通过时）/`lint --fix`（无需修复时）→ 静默
- `init`（有 .dev-doc 外副作用）/`lint --fix`（修了东西）/`iterate --confirm`/`test` → 输出额外信息

### 9. guard hook 更新

`check_devdoc_direct_create` 逻辑变更：

```
对于 .dev-doc/ 下的文件：
1. STATUS.yaml → 永远拦截（用 dow status set）
2. 结构型文件（task_*.md, issue_*.md, closed_issue_*.md, done_task_*.md, CHANGELOG.md）→ 永远拦截（不管是否存在）
3. 文档型文件（PRD.md, SPEC.md, BRAINSTORM.md）：
   - 不存在 → 拦截（用 dow <type> create）
   - 已存在 → 放行编辑
```

guard 提示消息也要更新为新命令名（`dow task create` 而非 `dow doc task`）。

### 10. confirm 机制统一

三个需要 confirm 的命令共享 token 生成逻辑，但前缀不同：

```rust
fn generate_token(prefix: &str, args_hash_input: &[&str]) -> String {
    // hash(cwd + minute + args) → prefix + 8位 hex
    format!("{}-{}", prefix, hex_hash[..6])
}
```

- `dow iterate` → ITR-xxxxxx
- `dow task reopen` → TRO-xxxxxx
- `dow issue reopen` → IRO-xxxxxx

confirm 时 dow 重新计算 token（使用 confirm 传入的参数），校验与传入 token 匹配。参数变更则 token 不匹配，返回错误。

### 11. Exit code 约定

| Code | 含义 |
|------|------|
| 0 | 成功 / lint 无问题 |
| 1 | 失败 / lint 有 warning 或 error |
| 2 | 用法错误（参数不对） |

### 12. Breaking Changes 迁移

由于 dow 不是公共 API（只有 agent 和项目内 hook 使用），不需要渐进式迁移：
- 一次性重构 cli.rs 的 Commands enum
- 同步更新 hooks.json 中 guard 提示消息
- 同步更新 `dow hooks context` 注入内容
- 同步更新 plugin/commands 中的 slash command 指令
- 同步更新 CLAUDE.md 文档

## Test Plan

- `cargo test` 全量通过（现有测试 + 新增测试）
- 新增集成测试：`tests/test_dow_task.rs`（create/list/show/done/reopen）
- 新增集成测试：`tests/test_dow_lint.rs`（合并后行为）
- 现有 `tests/test_dow_status_write.rs` 适配 `status set` 语法
- guard hook 测试覆盖新的文档型/结构型区分逻辑
- `dow test` 原有 shell 测试需适配新命令语法

## Self Check
- [x] Goal is clear
- [x] Acceptance criteria are testable
- [x] Matches current mode
