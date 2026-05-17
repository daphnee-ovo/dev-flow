# 头脑风暴记录 — dev-doc 结构重设计

**日期**：2026-05-16

## 背景与目的

一致性审查发现 dev-doc 存在多处结构性问题：session 目录定位模糊且从未被消费、TASK 单文件设计导致迭代膨胀、/done 和 /iterate 在 mvp 模式下行为不一致。本次重设计解决这些问题。

## 关键决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| session/ 目录 | 砍掉，用 CHANGELOG.md 替代 | session 内容已由主文档链覆盖，memory/ 从未被消费 |
| 会话记录持久化 | dev-doc/CHANGELOG.md 追加式 | 需要持久化但不需要复杂结构 |
| inject-context 附带历史 | 最近一条 CHANGELOG 记录 | 让新会话知道上次做了什么 |
| TASK 结构 | 拆分为 dev-doc/task/ 多文件 | 避免单文件膨胀、支持按批次管理 |
| task 文件命名 | task_\<YYYY-MM-DD\>_\<seq\>.md | 与 issue 风格一致，去掉 source |
| task 完成标记 | done_ 前缀（hook 自动重命名） | 文件内全部 checkbox 勾选时触发 |
| task 注入策略 | 按优先级分层，只展示当前优先级 | P0 全完成才展示 P1，控制注入量 |
| /done 适配 | 脚本根据 mode 确定检查项 | 检查项与该模式要求的阶段匹配 |
| /iterate 前置 | 自动触发 /done | 未 done 时先检查，通过后继续 iterate |
| archive 命名 | archive/v\<N\>-\<topic\>/ | iterate 时询问用户输入主题 |
| /issue 命令 | 新增，手动创建 issue | 补充自动发现之外的 bug 记录入口 |
| issue vs task 边界 | issue = 已实现代码的缺陷；task = 需要完成的工作 | 方案返工属于 task 调整，不产生 issue |
| STATUS.yaml phase | 去掉 MVP 值 | mvp 是 mode 不是 phase，用 /done + /iterate 收尾 |
| /brainstorm 可用性 | 所有模式可用，始终可选 | 自由探索工具，不属于必经阶段 |

## 设计方案

### 新目录结构

```
dev-doc/
├── STATUS.yaml
├── CHANGELOG.md                           # 会话记录（追加式）
├── BRAINSTORM.md                          # 持久，不归档
├── PRD.md
├── SPEC.md
├── TEST.md
├── task/                                  # 任务文件（按批次）
│   ├── task_2026-05-16_1.md
│   ├── task_2026-05-16_2.md
│   └── done_task_2026-05-16_1.md
├── issue/                                 # 问题追踪
│   ├── issue_test_2026-05-16_1.md
│   └── closed_issue_test_2026-05-16_1.md
└── archive/                               # 历史迭代
    └── v1-auth-system/
        ├── PRD.md
        ├── SPEC.md
        ├── TEST.md
        ├── task/
        │   └── done_task_2026-05-15_1.md
        └── issue/
            └── closed_issue_test_2026-05-15_1.md
```

### Task 文件格式

```markdown
---
title: TASK - <批次主题>
nums: <任务总数>
---

- [ ] T1：<标题>
  - level: P0
  - details：<描述>
  - depends on：<依赖>
  - Done when：<完成标准>
- [ ] T2：<标题>
  - level: P1
  - details：<描述>
  - depends on：<依赖>
  - Done when：<完成标准>
```

### inject-context 注入格式

**有未关闭 issue 时（优先展示 issue 标题）：**

```
[TASK] Total: 11 | P0:5 P1:4 P2:2
[Issue] Total: 3
Current Issue：
  - issue_test_2026-05-16_1: 登录接口返回 500
  - issue_devtest_2026-05-16_1: 缓存未失效
  - issue_other_2026-05-16_1: 并发写入丢数据
```

**无未关闭 issue 时（展示 task 标题）：**

```
[TASK] Total: 11 | P0:5 P1:4 P2:2
[Issue] Total: 0
Current Task：
  - [ ] T1: workflow.py 新增 review 子命令
  - [ ] T3: 实现缓存层
  ...（仅列出当前优先级中未完成的标题）
```

规则：
- 跨所有未关闭 task 文件汇总统计
- issue 和 task 标题互斥展示，优先级：issue > task
- 有 issue 时：列 issue 标题，task 只显示统计
- 无 issue 时：列当前优先级未完成的 task 标题（P0 全 done → P1 → P2）
- 不注入 details/done-when，agent 需要时自行 read 文件

### /done 模式适配

脚本根据 STATUS.yaml 的 mode 确定检查项：

| 模式 | 检查项 |
|------|--------|
| full | PRD + SPEC + task 全完成 + TEST 全过 + 无 P0 issue |
| quick | SPEC + task 全完成 + TEST 全过 + 无 P0 issue |
| fast | task 全完成 + TEST 全过 + 无 P0 issue |
| mvp | SPEC 存在 + 代码可运行（用户确认） |

### /iterate 流程

1. 如果 STATUS 不是 DONE → 自动触发 /done
2. /done 通过后继续 iterate
3. 询问用户本轮主题（用于 archive 命名）
4. 归档：done_task_* 和 closed_issue_* 移入 archive/v\<N\>-\<topic\>/
5. 未完成的 task 和未关闭的 issue 留在当前目录带入下一轮

### Issue 文件格式

```markdown
---
source: test | devtest | other
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

- checkbox 勾选 = 已关闭
- 文件内全部勾选 → hook 自动加 `closed_` 前缀
- issue 展示也按优先级分层：P0 全关闭才展示 P1 标题

### /issue 命令（新增）

手动创建 issue（source 为 other）。创建后提示是否需要 `/fix` 修复。

### CHANGELOG.md 格式

```markdown
# Changelog

## 2026-05-16
- 14:30 fix-login-bug: 修复登录验证逻辑

## 2026-05-15
- 14:00 implement-auth: 完成认证模块
- 10:00 init-project: 项目初始化
```

save-session hook 在 Stop 时追加一条记录。

### 归档策略

| 文件 | 归档规则 |
|------|----------|
| BRAINSTORM.md | 归档已完成且对后续推进无用的内容 |
| CHANGELOG.md | iterate 时全部归档（新迭代从空文件开始） |
| PRD.md / SPEC.md / TEST.md | iterate 时归档 |
| done_task_*.md | iterate 时归档 |
| closed_issue_*.md | iterate 时归档 |
| 未完成 task / 未关闭 issue | 留在当前目录带入下一轮 |

archive 主题命名应能反映该轮迭代的核心变化。

## 约束与边界

- 不做：多工程模式暂不改动（保持现有分支检测逻辑）
- 不做：task 文件内的格式大改（保持 checkbox + 子字段格式）

## 下一步

建议进入 `/spec` 细化技术实现方案（hook 脚本改动、命令文件更新、文件模板）。
