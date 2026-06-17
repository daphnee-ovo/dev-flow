# SPEC 文件格式规范

## 路径

`dev-doc/<branch>/SPEC.md`

每次迭代只有一份 SPEC；`/iterate` 时自动归档到 `dev-doc/archive.db`（SQLite），源文件删除。

## 按 mode 的必需章节

| 章节 | full | quick | fast | mvp |
|------|:----:|:-----:|:----:|:---:|
| Goal | ✓ | ✓ | ✓ | ✓ |
| Scope (In/Out) | ✓ | ✓ | — | Out of scope |
| Requirements Trace | ✓ | — | — | — |
| Design | ✓ | ✓ | — | — |
| Acceptance | ✓ | ✓ | ✓ | — |
| Risks | ✓ | — | — | — |
| Test Plan | ✓ | ✓ | ✓ | Smoke Test |

降级规则：
- **full**：保留完整结构
- **quick**：必须有 Goal、Scope、Design、Acceptance、Test Plan
- **fast**：必须有 Goal、Acceptance、Test Plan
- **mvp**：必须有 Goal、Out of scope、Smoke Test

## 模板

```markdown
# SPEC: <主题>

## Goal
<目标>

## Scope
### In
### Out

## Requirements Trace
| Req | AC | Notes |
| --- | --- | --- |
| PRD-FR-001 或 user-request | SPEC-AC-001 | ADDED / MODIFIED / REMOVED |

## Design
<必要方案。能短就短。>

## Acceptance
- SPEC-AC-001: <可测验收>
- SPEC-AC-002: <可测验收>

## Risks
- <风险和回退>

## Test Plan
- <最小验证方式>

```

## Requirements Trace 格式

用于追踪需求到验收的映射关系。

| 列 | 说明 |
|----|------|
| Req | 需求来源标识：`PRD-FR-xxx`（来自 PRD）或 `user-request`（用户直接要求） |
| AC | 对应的验收标识：`SPEC-AC-xxx` |
| Notes | 变更说明：`ADDED`（新增）/ `MODIFIED`（修改）/ `REMOVED`（移除）；可附带简短理由 |

## Acceptance (SPEC-AC-xxx) 格式

验收条件使用统一前缀 `SPEC-AC-` 加三位数字序号：

```
- SPEC-AC-001: <可测试的验收描述>
- SPEC-AC-002: <可测试的验收描述>
```

规则：
- 每条验收必须是**可测试**的（能给出 pass/fail 判定）
- 序号从 001 开始递增，不跳号
- 描述使用陈述句，说明期望行为或结果
- 不可出现"大概"、"合理"等模糊词

## Self Check 格式

用 checkbox 列表，供 SPEC 作者在完成后自检(只供自检，非spec应写入内容)：

```markdown
## Self Check
- [ ] 目标清楚
- [ ] 边界清楚
- [ ] 验收可测
- [ ] 与当前 mode 匹配
```

所有项勾选后方可进入 TASK 阶段。
