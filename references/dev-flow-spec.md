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
| `STATUS.yaml` | 阶段、模式、版本、时间戳 | 自动维护 |
| `CHANGELOG.md` | 追加式会话日志 | hook（save-changelog） |
| `BRAINSTORM.md` | 头脑风暴探索记录 | `/brainstorm` |
| `PRD.md` | 产品需求（MoSCoW 优先级） | `/prd` |
| `SPEC.md` | 技术规范（架构、接口、数据模型） | `/spec` |
| `TEST.md` | 测试报告（用例、结果） | `/test` |

### Task 文件命名

**格式**：`task_<YYYY-MM-DD>_<seq>.md`

- `YYYY-MM-DD`：创建日期
- `seq`：当天序号（从 1 开始）

**完成标记**：全部 checkbox 勾选后 hook 自动重命名为 `done_` 前缀

**示例**：
```
task/
├── task_2026-05-15_1.md                   # 活跃任务
├── task_2026-05-16_1.md                   # 活跃任务
└── done_task_2026-05-14_1.md              # 已完成
```

### Issue 文件命名

**格式**：`issue_<source>_<YYYY-MM-DD>_<seq>.md`

- `source`：产出来源，固定值为 `test` / `devtest` / `other`
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

**获取下一个序号**：
```bash
SOURCE="test"
DATE=$(date +%Y-%m-%d)
NEXT_SEQ=$(find "$DOC_ROOT/issue" -name "issue_${SOURCE}_${DATE}_*.md" -o -name "closed_issue_${SOURCE}_${DATE}_*.md" 2>/dev/null | grep -oP "${SOURCE}_${DATE}_\K\d+" | sort -n | tail -1 || echo 0)
NEXT_SEQ=$((NEXT_SEQ + 1))
FILENAME="issue_${SOURCE}_${DATE}_${NEXT_SEQ}.md"
```

**关闭 issue**：
```bash
# issue_test_2026-05-14_1.md → closed_issue_test_2026-05-14_1.md
mv "$DOC_ROOT/issue/issue_test_2026-05-14_1.md" "$DOC_ROOT/issue/closed_issue_test_2026-05-14_1.md"
```

### Archive 命名

**格式**：`archive/v<N>-<topic>/`

- `N`：迭代版本号，从 STATUS.yaml 的 iteration 字段读取
- `topic`：由用户在 `/iterate` 时指定，简短描述本轮主题
- 归档内容：done_task_*、已关闭 issue、CHANGELOG.md、PRD.md、SPEC.md、TEST.md
- 未关闭 issue 留在当前 `issue/` 带入下一轮
- BRAINSTORM.md 不归档（持久参考）

## 生命周期规则

| 文件 | 创建时机 | 更新时机 | 归档时机 |
|------|----------|----------|----------|
| STATUS.yaml | `/mode` 或第一个阶段命令 | 每个阶段转换、hook 自动更新 | 不归档（原地更新） |
| CHANGELOG.md | hook（save-changelog） | 每次会话结束追加 | `/iterate` 时归档 |
| BRAINSTORM.md | `/brainstorm` | brainstorm 过程中 | **不归档**（持久参考） |
| PRD.md | `/prd` | 用户反馈修改 | `/iterate` 时归档 |
| SPEC.md | `/spec` | 用户反馈修改 | `/iterate` 时归档 |
| task/*.md | `/task` | 开发中勾选、hook 自动重命名 | `/iterate` 时归档 done_task_* |
| TEST.md | `/test` | 重新测试时覆盖 | `/iterate` 时归档 |
| issue/*.md | `/test` `/devtest` `/issue` | `/fix` 关闭+重命名 | 已关闭的归档，未关闭的保留 |

## 初始化

首次创建 dev-doc 时：

```bash
mkdir -p dev-doc/{issue,task,archive}
```

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
