# Issue 文件格式规范

## 路径

`dev-doc/<branch>/issue/issue_<source>_<YYYY-MM-DD>_<seq>.md`

关闭标记：hook 自动重命名为 `closed_issue_<source>_<YYYY-MM-DD>_<seq>.md`

## 模板

```markdown
---
source: test | other | audit
nums: <issue 总数>
---

- [ ] ISSUE-I001：<标题>
  - severity: P0
  - location：<文件路径:行号>
  - description：<具体描述>
  - reproduce：<复现方法，可选>
  - files_modify: [<修改文件，可选>]
  - files_create: [<创建文件，可选>]
  - fix：<关闭时填写修复说明>
- [x] ISSUE-I002：<标题>
  - severity: P1
  - location：<文件路径:行号>
  - description：<描述>
  - fix：修改了缓存失效逻辑
```

## 字段说明

| 字段 | 值 | 说明 |
|------|-----|------|
| source | `test` / `other` / `audit` | 发现来源；Task 关闭测试也使用 `test` |
| nums | 数字 | 该文件中 issue 总数 |
| severity | `P0` / `P1` / `P2` | **必填**。P0=阻塞、P1=严重、P2=轻微 |
| location | 文件路径:行号 | 问题所在位置 |
| description | 文本 | 具体描述 |
| reproduce | 文本（可选） | 复现步骤 |
| files_modify | 字符串数组（可选） | 关联的修改文件 |
| files_create | 字符串数组（可选） | 关联的新增文件 |
| fix | 文本 | 关闭时填写修复说明 |

## 状态标记

- `- [ ]`：未关闭
- `- [x]`：已关闭（checkbox 勾选 = 问题已修复并验证）

## 完成规则

- 修复 issue 后需手动将 `[ ]` 改为 `[x]` 并填写 fix 字段，但不需要手动重命名文件（hook 自动完成）
- 文件内所有 checkbox 均为 `[x]` → `dow hooks post-write` 自动重命名为 `closed_` 前缀
- `/iterate` 时 `closed_issue_*.md` 自动归档到 `dev-doc/archive.db`（SQLite），源文件删除
- 未关闭的 issue 文件留在当前目录带入下一轮迭代

## 优先级展示

dow hooks context 按优先级分层：P0 全关闭才展示 P1 标题。

## 命名规则

- `source`：`test`（dow test 发现）/ `other`（手动创建）/ `audit`（审计发现）
- `seq`：当天该来源的序号，从 1 开始
- 创建新 issue：`dow issue create`（自动计算序号；格式可用 `dow issue schema` 查询）
