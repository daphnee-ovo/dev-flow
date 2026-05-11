---
description: 报告当前项目状态和进度
allowed-tools: Bash, Read
---

# STATUS — 项目状态报告

## 执行步骤

1. 检测项目模式（单工程/多工程）
2. 读取 `dev-doc/STATUS.md`
3. 读取 `dev-doc/TASK.md` 统计任务完成比例
4. 扫描 `dev-doc/issue/` 统计未关闭 issue
5. 扫描 `dev-doc/session/task/` 获取最近会话记录
6. 输出格式化状态报告

## 输出格式

```
[dev-flow] 项目状态报告
━━━━━━━━━━━━━━━━━━━━━━
项目名称：<name>
当前阶段：<phase>
更新时间：<date>

任务进度：[████░░░░░░] X/Y 完成
未关闭 Issue：N 个
最近动态：
  - <date>: <action>
  - <date>: <action>

建议下一步：<suggestion>
```

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.md" -path "*/*/STATUS.md" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

如果 `dev-doc/` 不存在，提示这是新项目，建议执行 `/prd` 开始。
