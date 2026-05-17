# Task 文件格式规范

## 路径

`dev-doc/task/task_<YYYY-MM-DD>_<seq>.md`

完成标记：hook 自动重命名为 `done_task_<YYYY-MM-DD>_<seq>.md`

## 模板

```markdown
---
title: TASK - <批次主题>
nums: <任务总数>
---

- [ ] T1：<标题>
  - level: P0
  - details：<描述>
  - depends on：无
  - Done when：<可验证的完成标准>
- [ ] T2：<标题>
  - level: P1
  - details：<描述>
  - depends on：T1
  - Done when：<完成标准>
- [x] T3：<标题>（已完成）
  - level: P0
  - details：<描述>
  - depends on：无
  - Done when：<完成标准>
```

## 字段说明

| 字段 | 说明 |
|------|------|
| title | yaml 头，批次主题 |
| nums | yaml 头，该文件中任务总数 |
| level | P0=阻塞 / P1=重要 / P2=可选 |
| details | 任务具体描述 |
| depends on | 前置依赖（可跨文件引用：`<文件名>:T<N>`） |
| Done when | 可验证的完成标准（必须客观具体） |

## 状态标记

- `- [ ]`：未完成
- `- [x]`：已完成

## 完成规则

- 文件内所有 checkbox 均为 `[x]` → hook 自动重命名为 `done_` 前缀
- 归档时 `done_task_*.md` 移入 `archive/v<N>-<topic>/task/`
- 未完成的 task 文件留在当前目录带入下一轮迭代

## 命名规则

- `seq`：当天的序号，从 1 开始
- 获取下一个序号：
  ```bash
  DATE=$(date +%Y-%m-%d)
  NEXT_SEQ=$(find "$DOC_ROOT/task" -name "task_${DATE}_*.md" -o -name "done_task_${DATE}_*.md" 2>/dev/null | grep -oP "${DATE}_\K\d+" | sort -n | tail -1 || echo 0)
  NEXT_SEQ=$((NEXT_SEQ + 1))
  ```
