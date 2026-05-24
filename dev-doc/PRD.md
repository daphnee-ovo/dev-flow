# PRD：工程版本管理同步

## 1. 背景

dev-flow 当前的版本号仅存在于 commit message 文本中，与 git 状态（tag、branch）脱节。`/iterate` 只操作文档归档，不触及 git 版本管理。导致：
- 无法机器化读取当前版本
- git tag 与工程迭代不同步
- 分支文档归属不明确

## 2. 目标与非目标

### 目标
- 工程版本（VERSION 文件）与 git tag 保持同步
- `/iterate` 统一执行交付检查 + 打 tag + bump 版本（废弃 `/done`）
- 分支感知：dev-doc 路径与当前分支关联

### 非目标
- 不做 semver 严格语义化（插件是文档驱动，不是 library）
- 不做 release changelog 自动生成（现有 CHANGELOG 机制已覆盖）
- 不做多分支并行文档管理（当前单分支工作流足够）

## 3. 功能需求

### Must Have
- [ ] VERSION 文件：单一真相源，脚本可读取
- [ ] `/iterate` 执行交付检查 + 归档 + commit & 创建 annotated tag + bump 版本
- [ ] 废弃 `/done` 命令，职责并入 `/iterate`
- [ ] 移除 STATUS.yaml 中的 iteration 字段，由 VERSION 文件替代

### Should Have
- [ ] `inject-context.sh` hook 输出中展示当前版本号
- [ ] `/status` 命令展示版本号和对应 git tag 状态

### Won't Have
- 手动版本号编辑入口（只通过流程命令自动管理）
- 多分支并行版本追踪

## 4. 用户故事

1. 开发者执行 `/iterate`，系统自动交付检查、归档、commit、打 tag `v2.2.0`、bump 到 `v2.3.0`
2. 任何脚本通过 `cat VERSION` 获取当前版本，无需解析 commit message
3. `inject-context.sh` 和 `/status` 实时展示版本及 tag 同步状态

## 5. 约束

- 版本格式：`<major>.<minor>.<patch>`（当前从 `2.1.0` 开始）
- `/iterate` 默认 bump minor（新迭代 = 新功能周期）
- 如需 bump major，用户在 `/iterate` 时手动指定

## 6. 成功指标

- git tag 与 VERSION 文件 100% 一致
- 任何 DONE 状态的项目都有对应 tag
- `/iterate` 后新版本号立即可通过 `cat VERSION` 获取
