# Issue 文件格式规范

## 路径

`dev-doc/issue/issue_<source>_<YYYY-MM-DD>_<seq>.md`

关闭标记：hook 自动重命名为 `closed_issue_<source>_<YYYY-MM-DD>_<seq>.md`

## 模板

```markdown
---
source: test | devtest | other | audit
nums: <issue 总数>
---

- [ ] I1：<标题>
  - severity: P0
  - location：<文件路径:行号>
  - description：<具体描述>
  - reproduce：<复现方法，可选>
  - fix：<关闭时填写修复说明>
- [x] I2：<标题>
  - severity: P1
  - location：<文件路径:行号>
  - description：<描述>
  - fix：修改了缓存失效逻辑
```

## 字段说明

| 字段 | 值 | 说明 |
|------|-----|------|
| source | `test` / `devtest` / `other` / `audit` | 发现来源 |
| nums | 数字 | 该文件中 issue 总数 |
| severity | `P0` / `P1` / `P2` | P0=阻塞、P1=严重、P2=轻微 |
| location | 文件路径:行号 | 问题所在位置 |
| description | 文本 | 具体描述 |
| reproduce | 文本（可选） | 复现步骤 |
| fix | 文本 | 关闭时填写修复说明 |

## 状态标记

- `- [ ]`：未关闭
- `- [x]`：已关闭（checkbox 勾选 = 问题已修复并验证）

## 完成规则

- 文件内所有 checkbox 均为 `[x]` → hook 自动重命名为 `closed_` 前缀
- 归档时 `closed_issue_*.md` 移入 `archive/v<N>-<topic>/issue/`
- 未关闭的 issue 文件留在当前目录带入下一轮迭代

## 优先级展示

inject-context 按优先级分层：P0 全关闭才展示 P1 标题。

## 命名规则

- `source`：`test`（/test 发现）/ `devtest`（/devtest 发现）/ `other`（手动创建）
- `seq`：当天该来源的序号，从 1 开始
- 获取下一个序号：
  ```bash
  SOURCE="test"
  DATE=$(date +%Y-%m-%d)
  NEXT_SEQ=$(find "$DOC_ROOT/issue" -name "issue_${SOURCE}_${DATE}_*.md" -o -name "closed_issue_${SOURCE}_${DATE}_*.md" 2>/dev/null | grep -oP "${SOURCE}_${DATE}_\K\d+" | sort -n | tail -1 || echo 0)
  NEXT_SEQ=$((NEXT_SEQ + 1))
  ```
