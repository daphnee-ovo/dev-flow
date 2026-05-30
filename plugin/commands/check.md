---
description: 检查开发工作是否已同步到 .dev-doc 文档
allowed-tools: Bash, Read
---

# CHECK — 文档同步检查

## 执行方式

直接运行脚本，展示输出：

```bash
dow check
```

脚本自动检查 CHANGELOG、task 完成度与 phase 匹配、issue 状态、代码变更 vs 文档更新时间、阶段必要文件。Agent 只需运行并展示结果。

## 注意

- 这是只读检查，不修改任何文件
- 输出建议但不自动执行
