# dev-doc 目录规范

## 目录结构

```
dev-doc/
├── STATUS.md                              # 项目状态
├── BRAINSTORM.md                          # 头脑风暴记录（持久，不归档）
├── PRD.md                                 # 产品需求文档
├── SPEC.md                                # 技术规范
├── TASK.md                                # 任务清单
├── TEST.md                                # 测试报告
├── issue/                                 # 问题追踪
│   ├── issue_<source>_<date>_<seq>.md          # 未关闭
│   └── closed_issue_<source>_<date>_<seq>.md   # 已关闭
├── session/                               # 会话记录
│   ├── <seq>-<topic>.md                   # 按序号+主题
│   └── memory/                            # 跨会话记忆
└── archive/                               # 历史迭代
    └── v<N>/                              # 按版本号
        ├── PRD.md
        ├── SPEC.md
        ├── TASK.md
        ├── TEST.md
        └── issue/
```

## 多工程模式

当项目有多分支并行开发时：

```
dev-doc/
├── main/
│   ├── STATUS.md
│   ├── ...
├── feature-auth/
│   ├── STATUS.md
│   ├── ...
```

检测规则：如果 `dev-doc/` 下存在包含 `STATUS.md` 的子目录，则进入多工程模式，`DOC_ROOT=dev-doc/<当前分支名>`。

## 文件命名规范

### 主文档

全大写，固定名称，不带日期：

| 文件 | 内容 | 产出命令 |
|------|------|----------|
| `STATUS.md` | 阶段、模式、版本、时间戳 | 自动维护 |
| `BRAINSTORM.md` | 头脑风暴探索记录 | `/brainstorm` |
| `PRD.md` | 产品需求（MoSCoW 优先级） | `/prd` |
| `SPEC.md` | 技术规范（架构、接口、数据模型） | `/spec` |
| `TASK.md` | 任务清单（checkbox 格式） | `/task` |
| `TEST.md` | 测试报告（用例、结果） | `/test` |

### Issue 文件

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

### Issue 文件内容格式

```markdown
---
source: test | devtest | other
modified_time: 2026-05-15_14_30_00
severity: P0 | P1 | P2
status: exist | fix | closed
task: <关联的任务名，如适用>
---

# <问题标题>

## 描述
<具体描述>

## 发现位置
<文件路径、函数名、接口等具体位置>

## 复现方法
<可选，如果问题可复现则填写>
1. ...
2. ...

## 修复记录
（status 变为 fix 时填写）
- 修复时间：
- 修改文件：
- 验证方式：
```

**yaml 头字段说明**：

| 字段 | 值 | 说明 |
|------|-----|------|
| `source` | `test` / `devtest` / `other` | 发现来源 |
| `modified_time` | `YYYY-MM-DD_HH_MM_SS` | 最后修改时间（每次状态变更更新） |
| `severity` | `P0` / `P1` / `P2` | P0=阻塞、P1=严重、P2=轻微 |
| `status` | `exist` / `fix` / `closed` | exist=发现，fix=已修复待验证，closed=验证通过 |
| `task` | 任务名或空 | 关联 TASK.md 中的任务 |

**状态流转**：
```
exist → fix → closed
  ↑       │
  └───────┘  （验证未通过，回退为 exist）
```

- `exist`：发现问题，由 `/test` `/devtest` 创建
- `fix`：开发修复后，由开发者或 `/fix` 标记
- `closed`：`/devtest` 或 `/test` 验证修复有效，重命名文件加 `closed_` 前缀

### Session 文件

**格式**：`<seq>-<topic>.md`

- `seq`：3 位数字序号，从 001 开始
- `topic`：简短描述本次会话主题（英文 kebab-case）

**示例**：
```
session/
├── 001-init-project.md
├── 002-implement-api.md
├── 003-fix-login-bug.md
└── memory/
    └── decisions.md              # 跨会话的关键决策记录
```

**获取下一个序号**：
```bash
NEXT_SEQ=$(find "$DOC_ROOT/session" -maxdepth 1 -name "*.md" 2>/dev/null | grep -oP '\d{3}' | sort -n | tail -1 || echo 0)
NEXT_SEQ=$(printf "%03d" $((10#$NEXT_SEQ + 1)))
```

### Archive 文件

**格式**：`archive/v<N>/`

- `N`：迭代版本号，从 1 开始
- 归档时复制主文档 + 已关闭 issue
- 未关闭 issue 留在当前 `issue/` 带入下一轮

## STATUS.md 格式

```markdown
# 项目状态

- 项目名称：<name>
- 当前阶段：<PRD | SPEC | TASK | DEV | TEST | DONE | MVP>
- 开发模式：<full | quick | fast | mvp>
- 当前迭代：v<N>
- 更新时间：YYYY-MM-DD HH:MM
- 迭代启动时间：YYYY-MM-DD HH:MM
```

## TASK.md 格式

```markdown
---
title: TASK - <project name> v<version>
nums: <task count>
---
# TASK 

## TASK LIST

- [ ] TASK 1：<描述>
  - level: P0 | P1 | P2
  - details：<详细描述>
  - depends on：<依赖的任务>
  - Done when：<可验证的完成标准>
- [ ] TASK 2：<描述>
  - level: P0 | P1 | P2
  - details：<详细描述>
  - depends on：<依赖的任务>
  - Done when：<可验证的完成标准>
- [x] TASK 3：<描述>（已完成）
  - level: P0 | P1 | P2
  - details：<详细描述>
  - depends on：<依赖的任务>
  - Done when：<可验证的完成标准>
```

## 临时文件

临时文件统一放在项目根目录 `tmp/` 下：

```
tmp/
├── debug_output.log
├── export_2026-05-15.csv
└── ...
```

- `tmp/` 应加入 `.gitignore`
- **禁止使用系统 `/tmp/` 目录**——所有临时文件必须放在项目内 `tmp/` 下，确保可追溯、可清理
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

### TEST.md 测试报告

`/test` 执行后产出 `dev-doc/TEST.md`，记录本次测试结果：

```markdown
# 测试报告

- 执行时间：YYYY-MM-DD HH:MM
- 测试范围：全量 / 指定模块
- 总用例数：N
- 通过：N
- 失败：N

## 失败用例

| 模块 | 用例 | 错误信息 | 关联 issue |
|------|------|----------|-----------|
| auth | test_login_with_invalid_email | AssertionError... | issue_test_2026-05-15_1 |

## 通过模块
- auth（12/12）
- api（8/8）
```

## 生命周期规则

| 文件 | 创建时机 | 更新时机 | 归档时机 |
|------|----------|----------|----------|
| STATUS.md | `/mode` 或第一个阶段命令 | 每个阶段转换、hook 自动更新 | 不归档（原地更新） |
| BRAINSTORM.md | `/brainstorm` | brainstorm 过程中 | **不归档**（持久参考） |
| PRD.md | `/prd` | 用户反馈修改 | `/iterate` 时归档 |
| SPEC.md | `/spec` | 用户反馈修改 | `/iterate` 时归档 |
| TASK.md | `/task` | 开发中勾选、/devtest 取消勾选 | `/iterate` 时归档 |
| TEST.md | `/test` | 重新测试时覆盖 | `/iterate` 时归档 |
| issue/*.md | `/test` `/devtest` 或手动 | `/fix` 追加关闭记录+重命名 | 已关闭的归档，未关闭的保留 |
| session/*.md | hook（save-session） | 会话中 | 不归档（持续积累） |

## 初始化

首次创建 dev-doc 时：

```bash
mkdir -p dev-doc/{issue,session/memory,archive}
```
