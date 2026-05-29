# dev-flow 流程规范

## 目录结构

```
dev-doc/
├── STATUS.yaml                              # 项目状态
├── CHANGELOG.md                           # 追加式会话日志
├── BRAINSTORM.md                          # 头脑风暴记录（持久，不归档）
├── PRD.md                                 # 产品需求文档
├── SPEC.md                                # 技术规范
├── TEST.md                                # 测试报告
├── task/                                  # 任务清单（多文件）
│   ├── task_<YYYY-MM-DD>_<seq>.md              # 活跃任务
│   └── done_task_<YYYY-MM-DD>_<seq>.md         # 已完成
├── issue/                                 # 问题追踪
│   ├── issue_<source>_<date>_<seq>.md          # 未关闭
│   └── closed_issue_<source>_<date>_<seq>.md   # 已关闭
└── archive/                               # 历史迭代
    └── v<N>-<topic>/                      # 按版本号+主题
        ├── PRD.md
        ├── SPEC.md
        ├── done_task_*.md
        ├── TEST.md
        ├── CHANGELOG.md
        └── issue/
```

## 多工程模式

当项目有多分支并行开发时：

```
dev-doc/
├── main/
│   ├── STATUS.yaml
│   ├── ...
├── feature-auth/
│   ├── STATUS.yaml
│   ├── ...
```

检测规则：如果 `dev-doc/` 下存在包含 `STATUS.yaml` 的子目录，则进入多工程模式，`DOC_ROOT=dev-doc/<当前分支名>`。

## 文件命名规范

### 主文档

全大写，固定名称，不带日期：

| 文件 | 内容 | 产出命令 |
|------|------|----------|
| `STATUS.yaml` | 阶段、模式、版本、时间戳 | `dow init` 创建，`dow status` 更新 |
| `CHANGELOG.md` | 追加式会话日志 | `dow hooks save-changelog` |
| `BRAINSTORM.md` | 头脑风暴探索记录 | `/brainstorm` |
| `PRD.md` | 产品需求（MoSCoW 优先级） | `/prd` |
| `SPEC.md` | 技术规范（架构、接口、数据模型） | `/spec` |
| `TEST.md` | 测试报告（用例、结果） | `/test` |

### Task 文件命名

**格式**：`task_<YYYY-MM-DD>_<seq>.md`

- `YYYY-MM-DD`：创建日期
- `seq`：当天序号（从 1 开始）

**完成标记**：全部 checkbox 勾选后 `dow hooks post-write` 自动重命名为 `done_` 前缀

**示例**：
```
task/
├── task_2026-05-15_1.md                   # 活跃任务
├── task_2026-05-16_1.md                   # 活跃任务
└── done_task_2026-05-14_1.md              # 已完成
```

### Task 文件内容格式

遵循 `references/dev-doc/TASK-FILE.md` 定义的格式。

**状态标记**：`- [ ]` 未完成，`- [x]` 已完成

**Complexity 与工作模型**：

`dow hooks context` 输出的 items 中嵌入 complexity 标记（如 `TASK-T002[M]: xxx`），用于指导 DEV 阶段选择执行模型：S→简单模型，M→正常或高级模型，L→高级模型。

**完成规则**：
- 文件内所有 checkbox 均为 `[x]` → `dow hooks post-write` 自动重命名为 `done_` 前缀
- 归档时 `done_task_*.md` 移入 `archive/v<N>-<topic>/`
- `/iterate` 时活跃 task 文件（`task_*.md`）也一并归档

### Issue 文件命名

**格式**：`issue_<source>_<YYYY-MM-DD>_<seq>.md`

- `source`：产出来源，固定值为 `test` / `devtest` / `other` / `audit`
- `YYYY-MM-DD`：创建日期
- `seq`：当天该来源的序号（从 1 开始，按 source+date 计数）

**关闭标记**：加 `closed_` 前缀

**示例**：
```
issue/
├── issue_test_2026-05-14_1.md              # /test 在 5月14日发现的第 1 个 issue
├── issue_test_2026-05-14_2.md              # /test 在 5月14日发现的第 2 个 issue
├── issue_devtest_2026-05-15_1.md           # /devtest 在 5月15日发现的第 1 个 issue
├── issue_other_2026-05-15_1.md             # 手动或其他来源创建
├── closed_issue_test_2026-05-14_1.md       # 已关闭
└── closed_issue_devtest_2026-05-15_1.md    # 已关闭
```

**创建新 issue 文件**：
```bash
dow doc issue --source <source>
```

**关闭 issue**：文件内所有 checkbox 勾选为 `[x]` 后，`dow hooks post-write` 自动重命名为 `closed_` 前缀。

### Archive 命名

**格式**：`archive/v<N>-<topic>/`

- `N`：当前版本号，从 VERSION 文件读取
- `topic`：由用户在 `/iterate` 时指定，简短描述本轮主题
- 归档内容：done_task_*、task_*（活跃任务）、已关闭 issue、CHANGELOG.md、PRD.md、SPEC.md、TEST.md
- 未关闭 issue 留在当前 `issue/` 带入下一轮
- BRAINSTORM.md 不归档（持久参考）

## 开发模式

### 标准模式

| 模式 | 流程 | 适用场景 |
|------|------|----------|
| `full` | PRD → SPEC → TASK → DEV → TEST → ITERATE | 完整需求周期 |
| `quick` | SPEC → TASK → DEV → TEST → ITERATE | 需求已明确 |
| `fast` | TASK → DEV → TEST → ITERATE | 方案已确定 |
| `mvp` | SPEC → TASK → DEV → ITERATE | 快速验证，跳过 TEST |

### audit 模式

**格式**：`audit/<previous>`（如 `audit/quick`、`audit/full`）

**触发规则**：
- 非 DEV 阶段创建 issue 文件（`issue/issue_*.md`）时，由 `dow hooks post-write` 自动触发
- 不支持手动通过 `/mode` 设置

**行为**：
- 进入 audit 模式时：保存当前 mode 至 `audit/<当前mode>`，将 phase 强制设为 DEV
- audit 模式下 DEV 阶段提示 `issue → /fix → /iterate 恢复原模式`
- `/iterate` 时跳过 task 完成度检查（因为 audit 模式可能没有 task）
- `/iterate` 完成后自动恢复为 `audit/` 后的原模式（如 `audit/quick` → 恢复为 `quick`）
- 恢复后 phase 按原模式规则重置（full→PRD, quick/mvp→SPEC, fast→TASK）

## VERSION 机制

### VERSION 文件

项目根目录下的 `VERSION` 文件是版本号的**单一真相源**（Single Source of Truth）。

- 格式：`major.minor.patch`（如 `2.6.1`）
- 纯文本文件，仅包含版本号字符串

### 版本操作（`/iterate` 时自动执行）

由 `dow iterate` 自动完成：归档、bump VERSION、commit、tag。

### dow hooks context 输出

JSON 格式，包含以下字段：

| 字段 | 说明 |
|------|------|
| version | VERSION 文件中的版本号 |
| version_tag | `synced`（git tag 已存在）/ `no-tag`（开发中） |
| mode | 当前开发模式 |
| phase | 当前阶段 |
| exec_mode | 执行模式（step/continuous） |
| doc_root | 文档根目录路径 |
| tasks | `{total, done, by_priority}` |
| issues | 未关闭 issue 数 |
| current_items | 当前最高优先级的 task 或 issue 列表 |
| last_changelog | 最近一条 CHANGELOG 条目 |

DEV 阶段无活跃 task 且无 open issue 时，输出 `{blocked: true, reasons: [...]}`。

## 生命周期规则

| 文件 | 创建时机 | 更新时机 | 归档时机 |
|------|----------|----------|----------|
| STATUS.yaml | `dow init` 或 `/mode` | 阶段转换时 `dow status` 更新 | 不归档（原地更新） |
| CHANGELOG.md | `dow hooks save-changelog` | 每次会话结束追加 | `dow iterate` 时归档 |
| BRAINSTORM.md | `/brainstorm` | brainstorm 过程中 | **不归档**（持久参考） |
| PRD.md | `/prd` | 用户反馈修改 | `dow iterate` 时归档 |
| SPEC.md | `/spec` | 用户反馈修改 | `dow iterate` 时归档 |
| task/*.md | `/task` | 开发中勾选、`dow hooks post-write` 自动重命名 | `dow iterate` 时归档 done_task_* 和 task_*（iterate 前阻断保证已全完成） |
| TEST.md | `/test` | 重新测试时覆盖 | `dow iterate` 时归档 |
| issue/*.md | `/test` `/devtest` `/issue` | `/fix` 修复后 `dow hooks post-write` 自动重命名 | 已关闭的归档，未关闭的保留 |

## 初始化

```bash
dow init --name <项目名> --mode <mode>
```

自动创建 `dev-doc/{issue,task,archive}`、`STATUS.yaml`、根目录 `VERSION`（多分支格式：`(<branch>)<semver>`）、`CHANGELOG.md`。

## 临时文件

临时文件统一放在项目根目录的本地临时目录下。若项目已有 `tmp` 或 `temp`，沿用已有目录；两者都不存在时，默认使用 `tmp`：

```
tmp 或 temp
├── debug_output.log
├── export_2026-05-15.csv
└── ...
```

- 选用的本地临时目录应加入 `.gitignore`
- **禁止写入系统临时目录**——所有临时文件必须放在项目内 `tmp` 或 `temp` 下，确保可追溯、可清理
- 不要在 `dev-doc/`、`tests/`、`src/` 中存放临时文件
- 会话结束或问题解决后可清理，不做持久化

## 测试代码规范

### 位置

测试代码统一放在项目根目录 `tests/` 下，按模块分子目录：

```
tests/
├── auth/
│   ├── test_login.py
│   ├── test_register.py
│   └── test_token.py
├── api/
│   ├── test_users.py
│   └── test_orders.py
├── db/
│   └── test_migrations.py
└── conftest.py                # 共享 fixtures（如适用）
```

### 命名规则

- 文件名：`test_<模块名>.py`
- 目录名：与源码模块对应（如 `src/auth/` → `tests/auth/`）
- 测试函数：`test_<行为描述>()`，如 `test_login_with_invalid_email()`

### 与 dev-flow 的关系

| 阶段 | 测试要求 |
|------|----------|
| `/devtest` | 运行与当前任务相关的测试模块 |
| `/test` | 运行全量测试 `tests/` |
| `/fix` | 修复后运行相关测试验证 |

### 测试文件创建时机

- SPEC 中定义了接口/行为 → TASK 阶段拆出"编写测试"任务
- DEV 阶段实现功能时同步编写测试
- `/devtest` 验证时**必须将测试代码写入 `tests/`**，不允许直接在终端运行临时命令验证
- `/test` 全量测试时运行 `tests/` 下所有测试
