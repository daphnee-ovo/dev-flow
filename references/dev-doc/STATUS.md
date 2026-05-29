# STATUS.yaml Schema

## 路径

`dev-doc/STATUS.yaml`

纯 YAML 格式，由 hook 自动维护，也可手动编辑。

## 必需字段

| 字段 | 类型 | 说明 |
|------|------|------|
| name | string | 项目标识名称 |
| phase | enum | 当前流程阶段 |
| mode | enum | 开发模式 |
| updated | datetime | 最近一次阶段变更时间（格式：`YYYY-MM-DD HH:MM`） |
| started | datetime | 迭代开始时间，由 `/iterate` 或首次 `/mode` 时记录 |

## phase 合法值

| 值 | 说明 |
|----|------|
| PRD | 需求阶段 |
| SPEC | 技术规范阶段 |
| TASK | 任务拆解阶段 |
| DEV | 开发阶段 |
| TEST | 测试阶段 |
| DONE | 迭代完成 |

流转顺序：`PRD → SPEC → TASK → DEV → TEST → DONE`

## mode 合法值

| 值 | 说明 |
|----|------|
| full | 完整流程，所有阶段文档齐全 |
| quick | 精简流程，省略部分文档章节 |
| fast | 快速流程，仅保留核心验收 |
| mvp | 最小可行，只关注目标和冒烟测试 |
| audit/\<previous\> | 审计模式，`/` 后跟进入 audit 前的原始 mode |

audit 模式示例：`audit/full`、`audit/quick`

## 可选字段

| 字段 | 类型 | 说明 |
|------|------|------|
| exec_mode | enum | 执行模式：`step`（逐步确认）/ `continuous`（连续执行） |
| goals_minor | string | 当前 minor 版本的目标描述（agent 判断是否满足 bump minor 条件的依据） |
| goals_major | string | 下一个 major 版本的目标描述（agent 判断是否满足 bump major 条件的依据） |

## 版本目标说明

goals 字段为 agent 提供版本升级的判断依据：
- agent 在 iterate 时检查当前交付是否满足 minor/major 目标，如满足则建议 `-v minor` 或 `-v major`
- 用户也可在 iterate 时直接指定 `-v minor/major`
- 默认 iterate 只 bump patch，patch 不打 git tag
- minor/major 自动打 git tag

设置方式：`dow status --goals-minor "xxx"` / `dow status --goals-major "xxx"`

## 示例

```yaml
name: my-project
phase: DEV
mode: full
goals_minor: dow 完善与优化
goals_major: 插件生态与多平台支持
updated: 2026-05-26 10:30
started: 2026-05-26 09:00
```

带可选字段的完整示例：

```yaml
name: my-project
phase: TEST
mode: audit/quick
exec_mode: continuous
goals_minor: 性能优化与缓存策略
updated: 2026-05-26 14:00
started: 2026-05-20 09:00
```
