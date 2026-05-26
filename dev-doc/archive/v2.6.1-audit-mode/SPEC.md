# SPEC: Audit Mode 自动审计模式

## Goal

实现临时覆盖模式 `audit`，当审计（人或 agent）发现 issue 时自动进入，走 ISSUE → DEV(fix) → ITERATE 闭环后自动恢复原模式。

## Scope

### In
- STATUS.yaml mode 字段支持 `audit/<previous>` 格式
- post-write hook 检测 issue 创建自动触发 audit 模式
- iterate 完成后自动恢复原 mode
- audit 模式下跳过 task 完成度检查
- inject-context 展示 audit 状态
- mode.sh 拒绝手动设置 audit
- 相关文档（mode.md、iterate.md、fix.md、README、CLAUDE.md）同步

### Out
- 不做 `/audit` slash command（不需要手动触发入口）
- 不做 audit 专用的 issue 模板
- 不做 audit 历史记录/统计

## Requirements Trace

| Req | AC | Notes |
| --- | --- | --- |
| user-request: 审计发现 issue 自动进入 audit | SPEC-AC-001 | 核心触发逻辑 |
| user-request: mode 格式 audit/previous | SPEC-AC-002 | 数据模型 |
| user-request: iterate 后自动恢复 | SPEC-AC-003 | 恢复逻辑 |
| user-request: 不需要手动切换 | SPEC-AC-004 | 拒绝手动 + 自动触发 |

## Design

### 数据模型

```yaml
# 正常
mode: quick

# audit 激活
mode: audit/quick
phase: DEV

# iterate 后恢复
mode: quick
phase: SPEC  # 按原 mode 决定
```

解析：`cut -d'/' -f1` → 有效模式，`cut -d'/' -f2` → 原始模式。

### 触发条件（post-write.sh）

```
条件与：
  1. 写入文件匹配 dev-doc/*/issue/issue_*.md 或 dev-doc/issue/issue_*.md
  2. 当前 mode 不是 audit/*
  3. 当前 phase 不是 DEV
```

触发后：`mode` → `audit/<current>`，`phase` → `DEV`。

### 恢复逻辑（iterate.sh）

```
if effective_mode == "audit":
  original = mode.split('/')[1]
  mode = original
  phase = initial_phase_for(original)
  跳过 task 完成度检查
```

### 辅助函数（common.sh）

```bash
is_audit_mode()    # 判断 mode 是否以 audit/ 开头
enter_audit_mode() # 写入 audit/<current> + phase=DEV
```

### 守卫（mode.sh）

输入以 `audit` 开头 → 拒绝，退出码 1。

### 错误处理

| 场景 | 处理 |
|------|------|
| 已是 audit/X，再创建 issue | 不重复触发 |
| audit/audit/quick 嵌套 | is_audit_mode 阻止二次进入 |
| audit/ + 无效原 mode | 恢复为 quick（安全默认） |
| 用户手动 `/mode quick` | 允许覆盖，视为主动退出 |

## Acceptance

- SPEC-AC-001: 非 DEV 阶段创建 issue 文件后，STATUS.yaml mode 自动变为 `audit/<原mode>`、phase 变为 `DEV`
- SPEC-AC-002: `mode: audit/quick` 格式可被 inject-context 正确解析和展示
- SPEC-AC-003: audit 模式下 iterate 完成后，mode 恢复为原值，phase 按原 mode 重置
- SPEC-AC-004: `bash mode.sh audit` 被拒绝（退出码 1）；常规模式不受影响

## Risks

- **误触发**：非审计场景下创建 issue 也会触发 → 代价低（`/mode X` 可退出，不损坏数据）
- **嵌套 audit**：已有 is_audit_mode 守卫阻止
- **iterate 无 task**：audit 模式跳过 task 检查，仅要求 P0 issue 关闭

## Test Plan

- `tests/test_audit_mode.sh` 覆盖：
  - is_audit_mode / enter_audit_mode 函数行为
  - mode.sh 拒绝 audit 输入
  - post-write 触发条件（3 种场景）
  - iterate 恢复逻辑（audit/quick → quick, audit/fast → fast）
  - iterate 跳过 task 检查
  - inject-context 输出格式
- 现有 `tests/test_iterate.sh` 回归通过

## Self Check

- [x] 目标清楚
- [x] 边界清楚
- [x] 验收可测
- [x] 与当前 mode（quick）匹配
