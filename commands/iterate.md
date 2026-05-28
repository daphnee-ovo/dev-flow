---
description: 迭代交付 — 交付检查 + 归档 + commit & tag + bump 版本
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# ITERATE — 迭代交付

## 总则

`/iterate` 是 dev-flow 的迭代收尾命令。执行后完成当前版本的交付并开启下一个迭代。
包含原 `/done`（交付检查）和原 `/iterate`（归档 + 重置）的全部职责。

## 前置检查（阻断）

`dow iterate` 自动执行，任一不通过则停止：
1. task 文件中所有任务必须全部勾选 `[x]`（audit 模式跳过）
2. 无未关闭的 P0 issue
3. VERSION 文件存在且格式合法

## 参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--topic` | 归档主题（用于归档目录命名） | 必填 |
| `--type` | commit 类型（feat/fix/refactor/docs/perf/test/style/workflow） | 必填 |
| `--files` | 额外提交的文件/目录列表（空格分隔） | 可选 |
| `-v`/`--bump` | 版本递增类型：major/minor/patch | minor |
| `--confirm` | 确认执行（需配合环境变量 token） | - |

## 执行流程

### 阶段 1：预览（不带 --confirm）

```bash
dow iterate --topic <topic> --type <type> [--files f1 f2...] [-v minor]
```

输出预览信息：归档内容、版本号、将打的 tag、提交文件列表、确认 token。

### 阶段 2：确认执行（带 --confirm + 环境变量）

```bash
DOW_ITERATE_<token>=1 dow iterate --confirm --topic <topic> --type <type> [--files f1 f2...]
```

Token 通过环境变量前缀传递，有效期 5 分钟。确认后依次执行：

1. **归档** — 解析 task_*、done_task_*、closed_issue_*、PRD.md、SPEC.md、TEST.md、CHANGELOG.md 并写入 `dev-doc/archive.db`（SQLite），然后删除源文件
2. **重置 CHANGELOG** — 清空为 `# Changelog\n`
3. **git commit + tag** — `git add -u` + 显式 add 指定文件和 archive.db，commit message 格式为 `<type>: Release v<版本> <topic>`，CHANGELOG 条目作为 commit body
4. **bump 版本** — 递增版本号写入 VERSION
5. **重置 phase** — 按 mode 确定新迭代初始阶段

## Commit Message 格式

```
<type>: Release v<版本> <topic>

- <CHANGELOG 条目 1>
- <CHANGELOG 条目 2>
...
```

## Bump 类型决策

1. 默认 minor（每次迭代 = 新功能周期）
2. 用户指定 `--major` → major
3. Agent 检测到架构重构/破坏性变更 → 推荐 major，询问用户确认

## 执行方式

```bash
# 预览
dow iterate --topic "<topic>" --type <type> --files <file1> <file2>

# 确认执行（token 从预览输出获取）
DOW_ITERATE_<token>=1 dow iterate --confirm --topic "<topic>" --type <type> --files <file1> <file2>
```

agent 在调用前：
1. 询问用户本轮迭代主题和 commit 类型
2. 判断 bump 类型（默认 minor，检测到大变更推荐 major）
3. 运行预览，展示摘要输出
4. 获取用户确认后，使用 token 执行完整流程

## audit 模式行为

当 `mode` 为 `audit/xxx` 格式时（即通过 `/mode audit` 进入的审计模式）：

1. **跳过 task 完成度检查** — audit 模式下允许在任务未全部完成时执行 iterate
2. **P0 issue 检查仍然保留** — 即使是 audit 模式，也必须关闭所有 P0 issue 后才能 iterate
3. **iterate 完成后自动恢复原模式** — 从 `audit/xxx` 中提取原始模式 `xxx`，写回 STATUS.yaml 的 mode 字段，并按该模式确定新迭代的起始 phase（如 `audit/quick` → 恢复为 `quick`，phase 重置为 SPEC）
4. 如果原始模式无效或为空，默认恢复为 `quick`

## 注意

- 归档写入 SQLite（`dev-doc/archive.db`），源文件删除，iterate 后 dev-doc/ 中不残留 PRD/SPEC/TEST/CHANGELOG
- 如果 SQLite 中已存在同版本记录，说明重复操作，INSERT OR IGNORE 跳过
- `git add -u` 仅处理已跟踪文件的修改/删除，新文件需通过 `--files` 显式指定
- 查询历史归档使用 `dow archive` 子命令（list/show/tasks/issues/doc/stats）

## 完成后输出

```
[dev-flow] 迭代完成
━━━━━━━━━━━━━━━━━━━━━━
交付版本：v2.2.0 (tagged)
新版本：v2.3.0
阶段重置：SPEC
```
