---
description: 检查开发工作是否已同步到 dev-doc 文档
allowed-tools: Bash, Read
---

# CHECK — 文档同步检查

## 执行步骤

1. 检测项目模式，确定 DOC_ROOT
2. 获取最近代码变更时间（git log 最新 commit 时间）
3. 获取 dev-doc 下各文档的最后修改时间
4. 对比分析，报告哪些文档可能滞后

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.md" -path "*/*/STATUS.md" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

## 检查项

### 1. STATUS.md 时效性
- 读取 STATUS.md 中的"更新时间"字段
- 与当前时间对比，超过当天未更新则标记

### 2. TASK.md 与代码一致性
- 统计已勾选任务数和总任务数
- 检查最近 commit 是否有对应的任务勾选（最近有代码提交但无新勾选 → 可能遗漏）

### 3. Issue 处理状态
- 统计未关闭 issue 数量
- 如果有未关闭 issue 但最近 commit 未涉及修复 → 提醒

### 4. TEST.md 同步
- 如果阶段为 TEST 或 DONE，检查 TEST.md 是否存在且非空
- 如果所有任务已完成但无 TEST.md → 提醒应进入测试

### 5. Session 记录
- 检查 `dev-doc/session/` 下是否有今天的会话记录
- 如果有大量代码变更但无会话记录 → 提醒

## 输出格式

```
[dev-flow] 文档同步检查
━━━━━━━━━━━━━━━━━━━━━━
当前阶段：<phase>
最近代码提交：<time>
STATUS.md 更新：<time>

检查结果：
  ✓ STATUS.md — 已同步
  ✓ TASK.md — 3/5 完成，与代码一致
  ✗ Issue — 2 个未关闭，最近提交未涉及修复
  ✗ TEST.md — 所有任务已完成但未进入测试阶段
  ✓ Session — 今日有记录

建议：
  - <具体建议，如"执行 /fix 处理未关闭 issue">
  - <具体建议，如"执行 /test 进入项目测试">
```

## 判定规则

| 检查项 | 通过条件 | 不通过条件 |
|--------|----------|------------|
| STATUS.md | 今天有更新 | 超过一天未更新 |
| TASK.md | 最近 commit 后有对应勾选 | 有 commit 但无新勾选 |
| Issue | 无未关闭 issue，或最近 commit 含修复 | 有未关闭 issue 且长期未处理 |
| TEST.md | 阶段匹配（DEV 阶段可无） | 全部任务完成但无 TEST.md |
| Session | 今日有变更则有记录 | 有变更但无记录 |

## 注意

- 这是只读检查，不修改任何文件
- 输出建议但不自动执行
- 如果 dev-doc 不存在，提示"未初始化项目，建议执行 /init"
