---
description: 自动读取未关闭 issue 并修复
allowed-tools: Agent, Bash, Read, Write, Edit
---

# FIX — 自动修复未关闭 Issue

## 前置检查

1. 确认 `dev-doc/` 存在
2. 确认 STATUS 为 DEV
3. 扫描 `dev-doc/issue/` 目录，确认存在未关闭的 issue（即 `issue_*.md` 且不含 `closed_` 前缀）
4. 如果没有未关闭 issue，告知用户"当前没有待修复的 issue"并退出

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.yaml" -path "*/*/STATUS.yaml" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi
```

## 执行步骤

1. 列出所有未关闭 issue：
```bash
find "$DOC_ROOT/issue" -name "issue_*.md" ! -name "closed_issue_*.md" 2>/dev/null | sort
```

2. 逐一读取每个 issue 文件内容

3. 生成项目上下文：`bash "${CLAUDE_PLUGIN_ROOT}/scripts/lib/context.sh"`

4. 对每个 issue 启动独立 Agent 修复（如果 issue 间无依赖关系，可并行）

5. 修复完成后验证并关闭 issue

## Agent 调度（隔离模板）

**对每个未关闭 issue，启动独立修复子代理。按当前运行时调度：Claude Code 使用 `Agent`，Codex 使用 `spawn_agent`。子代理 prompt 必须使用以下内容：**

```
description: "FIX - 修复 issue: <issue 标题>"
prompt: `你是一名高级开发工程师。你的任务是修复以下 issue。

## Issue 内容

<粘贴 issue 文件的完整内容>

## 相关规范

<从 SPEC.md 中摘取与该 issue 相关的部分>

## 项目上下文

<执行 scripts/lib/context.sh 的输出，原样粘贴>

## 修复要求

1. 定位问题根因，不要只修表面症状
2. 修复代码必须符合 SPEC.md 的技术规范
3. 修复后必须实际运行验证（启动服务/执行命令/运行测试）
4. 确保修复不会引入新的问题（回归）
5. 修复范围最小化，不要顺带重构无关代码

## 输出格式

结论：已修复 / 无法修复
修改文件：<列出修改的文件路径>
验证方式：<如何验证修复成功>
原因：<如果无法修复，说明原因和建议>

## 禁止

- 不要阅读无关的历史文件
- 不要修改与 issue 无关的代码
- 不要添加 SPEC 未要求的新功能
- 不要修改其他 issue 的相关代码（避免冲突）
- 禁止写入系统临时目录；项目内 tmp 和 temp 都允许，已有目录优先，新项目默认 tmp`
```

## 输入隔离规则

| 允许传入 | 禁止传入 |
|----------|----------|
| 该 issue 的完整内容 | 其他 issue 的内容 |
| SPEC.md 中相关部分 | 开发过程对话历史 |
| task/ 中相关任务描述 | 无关历史记录 |
| 项目上下文（context.sh 输出） | PRD.md |

## 结果处理

- **已修复** → 将 issue 文件中对应条目勾选为 `[x]`，在 fix 字段填写修复说明。当文件中所有条目均为 `[x]` 时，重命名加 `closed_` 前缀（如 `issue_test_2026-05-15_1.md` → `closed_issue_test_2026-05-15_1.md`）

- **无法修复** → 保持 issue 打开状态，向用户报告原因和建议

## P0 issue 关闭时自动 bump

当一个包含 P0 severity 条目的 issue 文件被完全关闭时，自动执行 minor 版本 bump：

```bash
source "${CLAUDE_PLUGIN_ROOT}/scripts/lib/version.sh"
VER=$(version_read)
NEW_VER=$(version_bump "$VER" minor)
version_write "$NEW_VER"
git add VERSION
git commit -m "Bump to v${NEW_VER}: P0 issue fixed"
```

判断条件：被关闭的 issue 文件中存在 `severity: P0` 的条目。非 P0 issue 关闭时不触发 bump。

## 完成后

汇总所有 issue 的处理结果：
```
[dev-flow] Issue 修复报告
━━━━━━━━━━━━━━━━━━━━━━
已修复：N 个
无法修复：M 个

详情：
  ✓ <issue-1>: <一句话描述修复内容>
  ✓ <issue-2>: <一句话描述修复内容>
  ✗ <issue-3>: <无法修复原因>

建议下一步：<如果有无法修复的 issue，给出建议>
```

## 为什么每个 issue 独立 Agent

- 避免修复之间相互干扰（一个 fix 引入另一个 bug）
- 隔离上下文，让每个 Agent 专注于单一问题
- 支持并行修复，提升效率
- 修复失败不影响其他 issue 的处理
