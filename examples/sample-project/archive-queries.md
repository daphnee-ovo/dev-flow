# Archive Query Output

These outputs were generated from `examples/sample-project/.dev-doc/archive.db` in the isolated sample project.

## `dow archive list`

```json
[
  {
    "version": "0.1.0",
    "topic": "settings-page",
    "branch": "main",
    "released_at": "2026-06-08",
    "tasks": 3,
    "issues": 1
  }
]
```

## `dow archive show 0.1.0`

```json
{
  "version": "0.1.0",
  "topic": "settings-page",
  "tasks": [
    {
      "task_id": "TASK-T001",
      "title": "Add settings data model",
      "priority": "P0",
      "completed": true
    },
    {
      "task_id": "TASK-T002",
      "title": "Add settings validation tests",
      "priority": "P1",
      "completed": true
    },
    {
      "task_id": "TASK-T003",
      "title": "Document verification result",
      "priority": "P2",
      "completed": true
    }
  ],
  "issues": [
    {
      "issue_id": "ISSUE-I001",
      "title": "Settings validation accepted an empty theme",
      "severity": "P1",
      "resolved": true
    }
  ],
  "has_prd": false,
  "has_spec": false,
  "has_test": true,
  "has_brainstorm": false
}
```

## `dow archive tasks --version 0.1.0`

```json
[
  {
    "task_id": "TASK-T001",
    "title": "Add settings data model",
    "priority": "P0",
    "completed": true
  },
  {
    "task_id": "TASK-T002",
    "title": "Add settings validation tests",
    "priority": "P1",
    "completed": true
  },
  {
    "task_id": "TASK-T003",
    "title": "Document verification result",
    "priority": "P2",
    "completed": true
  }
]
```

## `dow archive issues --version 0.1.0`

```json
[
  {
    "issue_id": "ISSUE-I001",
    "title": "Settings validation accepted an empty theme",
    "severity": "P1",
    "resolved": true
  }
]
```

## `dow archive doc 0.1.0 TEST`

```markdown
# 测试报告

- 执行时间：2026-06-08 10:30
- 测试范围：settings module
- 总用例数：2
- 通过：2
- 失败：0

## 失败用例

| 模块 | 用例 | 错误信息 | 关联 issue |
|------|------|----------|-----------|
| settings | empty theme validation | fixed before archive | ISSUE-I001 |

## 通过模块

- settings defaults
- settings validation
```
