# BRAINSTORM 文件格式规范

## 路径

`dev-doc/<branch>/BRAINSTORM.md`

每次迭代只有一份 BRAINSTORM；`/iterate` 时自动归档到 `dev-doc/archive.db`（SQLite），源文件删除。

## 模板

```markdown
# 头脑风暴记录 — <主题>

**日期**：<today>

## 背景与目的
<为什么要做这件事>

## 关键决策
| 决策点 | 选择 | 理由 |
|--------|------|------|
| ... | ... | ... |

## 设计方案

### 架构
<系统整体结构>

### 组件
<各单元职责和接口>

### 数据流
<数据如何流动>

### 错误处理
<异常场景处理策略>

## 约束与边界
<明确不做什么>

## 下一步
<建议进入 /prd 还是直接 /spec>
```

## 说明

- `/brainstorm` 执行后产出，记录协作探索的结论
- `/iterate` 时自动归档到 SQLite，源文件删除（查询历史：`dow archive doc <ver> BRAINSTORM`）
- 后续 /prd 或 /spec 可读取作为输入
- 每次新的 brainstorm 覆盖旧内容（一个项目只有一份活跃 BRAINSTORM）
