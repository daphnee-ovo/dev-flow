---
description: 手动创建 issue
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# ISSUE — 手动创建问题记录

## 模式检测

`DOC_ROOT` 通过 `dow status --field doc_root` 获取。

## 执行步骤

### 1. 收集信息

询问用户（可从参数获取）：
- 问题标题
- 严重程度（P0/P1/P2）
- 发现位置（文件路径:行号）
- 描述

### 2. 确定文件

检查是否已有当天 `other` 来源的 issue 文件：

```bash
dow issue --list | grep 'other'
```

- 如果有且文件内 issue 数量合理（<10 个）→ 追加到现有文件
- 否则 → 创建新文件

### 3. 新建文件

```bash
dow doc --issue --source other
```

### 4. 写入格式

读取 `references/dev-doc/ISSUE.md` 获取完整格式定义。

### 5. 提示下一步

```
[dev-flow] Issue 已创建：dev-doc/issue/<filename>
是否需要立即修复？执行 /fix 自动修复未关闭 issue。
```

## 追加到现有文件

如果追加到已有文件：
1. 读取现有文件的 `nums` 值
2. 新 issue 编号为 `I<nums+1>`
3. 追加 checkbox 条目到文件末尾
4. 更新 frontmatter 中的 `nums`

## 注意

- 主 agent 直接执行，不启动子 agent
- source 固定为 `other`（区别于 test/devtest 自动创建）
- 创建后不自动修复，由用户决定是否 /fix
