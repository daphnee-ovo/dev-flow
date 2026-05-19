---
source: test
nums: 30
---

## 脚本缺陷

- [x] BUG#1: inject-context.sh 中 `grep -c` 返回 0 时退出码为 1，`|| echo 0` 导致算术展开收到 "0\n0"
  - severity: P0
  - files: scripts/hooks/inject-context.sh, scripts/commands/status.sh, scripts/commands/check.sh
  - fix: 所有脚本改用 `$(grep -c ... 2>/dev/null) || true; VAR=${VAR:-0}` 模式

- [x] BUG#2: inject-context.sh awk 中 getline 只读一行，但 level: 字段不一定紧跟标题行
  - severity: P0
  - files: scripts/hooks/inject-context.sh
  - fix: awk 循环 getline 最多 4 行，遇到下一个 checkbox 行则 break

## P0 文档不一致

- [x] commands/prd.md 仍引用 `mkdir -p dev-doc/session dev-doc/memory`（已废弃目录）
  - severity: P0
  - files: commands/prd.md
  - fix: 改为 mkdir -p dev-doc/{task,issue,archive}

- [x] commands/devtest.md 使用旧的 issue 输出格式（非 checkbox 批量格式）
  - severity: P0
  - files: commands/devtest.md
  - fix: 更新为 frontmatter + checkbox 格式

- [x] README.md 和 README.zh-CN.md 目录树包含 session/memory，缺少 task/issue/archive
  - severity: P0
  - files: README.md, README.zh-CN.md
  - fix: 更新目录树为 task/issue/archive/CHANGELOG 结构

## P1 文档不一致

- [x] commands/fix.md 引用 STATUS.yaml 中不存在的 `blocked_by` 字段
  - severity: P1
  - files: commands/fix.md
  - fix: 已移除，改为 checkbox 勾选流程

- [x] SKILL.md 命令表缺少 /brainstorm 命令
  - severity: P1
  - files: skills/dev-flow/SKILL.md, .claude/skills/dev-flow/SKILL.md, .agents/skills/dev-flow/SKILL.md
  - fix: 已确认存在，非问题

- [x] README.md 命令表缺少 /issue 命令
  - severity: P1
  - files: README.md, README.zh-CN.md
  - fix: 已添加 /issue 行

- [x] CONTRIBUTING.md 引用 session/ 目录和旧版流程
  - severity: P1
  - files: CONTRIBUTING.md
  - fix: 更新目录树为新结构

- [x] test-agent.md 内部矛盾：第 86 行说"合并到一个文件"，第 130 行说"每个问题单独文件"
  - severity: P1
  - files: agents/test-agent.md
  - fix: 统一为"同一次测试的问题写入同一个 issue 文件"

- [x] commands/done.md 检查清单引用旧目录结构
  - severity: P1
  - files: commands/done.md
  - fix: 修复 grep -c 模式

- [x] references/dev-flow-spec.md hook 表中仍有 save-session.sh
  - severity: P1
  - files: references/dev-flow-spec.md
  - fix: 已确认不存在，非问题

- [x] hooks.json/hooks/hooks.json 注册的 hook 脚本路径与实际不一致
  - severity: P1
  - files: hooks.json, hooks/hooks.json
  - fix: 已确认两者均正确使用 save-changelog.sh

- [x] commands/init.md 仍引用 session/ 目录创建
  - severity: P1
  - files: commands/init.md
  - fix: 已修正为 issue/task 文件

- [x] agents/prd-agent.md 输出部分引用 session/ 目录
  - severity: P1
  - files: agents/prd-agent.md
  - fix: 已确认无 session 引用，非问题

- [x] agents/spec-agent.md 禁止列表引用 session/ 目录
  - severity: P1
  - files: agents/spec-agent.md
  - fix: 已确认无 session 引用，非问题

- [x] SKILL.md DEV 阶段输入写的 "task/*.md + SPEC.md" 与 commands/fix.md 不一致
  - severity: P1
  - files: skills/dev-flow/SKILL.md
  - fix: 修正 TEST 行为 SPEC.md + task/*.md

- [x] commands/iterate.md 仍引用 session/ 归档逻辑
  - severity: P1
  - files: commands/iterate.md
  - fix: 已确认无 session 引用，非问题

- [x] commands/task.md agent prompt 与 agents/task-agent.md 格式定义不一致
  - severity: P1
  - files: commands/task.md
  - fix: 已确认格式一致，非问题

## P2 文档清理

- [x] commands/spec.md agent prompt 中 "不要阅读 dev-doc/session/" 改为 "不要阅读无关历史"
  - severity: P2
  - files: commands/spec.md
  - fix: 已更新

- [x] commands/devtest.md agent prompt 中 "不要阅读 dev-doc/session/" 改为通用表述
  - severity: P2
  - files: commands/devtest.md
  - fix: 已更新

- [x] commands/fix.md agent prompt 中引用 session/ 目录
  - severity: P2
  - files: commands/fix.md
  - fix: 已更新

- [x] commands/test.md agent prompt 中 "不要查看 git log" 可保留但 session/ 引用需清理
  - severity: P2
  - files: commands/test.md
  - fix: 已确认无 session 引用

- [x] agents/test-agent.md "不要阅读 dev-doc/session/" 改为通用表述
  - severity: P2
  - files: agents/test-agent.md
  - fix: 已确认无 session 引用

- [x] commands/brainstorm.md 中可能残留旧目录引用
  - severity: P2
  - files: commands/brainstorm.md
  - fix: 已确认无残留

- [x] commands/check.md 脚本调用路径需确认与实际一致
  - severity: P2
  - files: commands/check.md
  - fix: 路径正确

- [x] commands/mode.md 脚本调用路径需确认与实际一致
  - severity: P2
  - files: commands/mode.md
  - fix: 路径正确

- [x] commands/status.md 脚本调用路径需确认与实际一致
  - severity: P2
  - files: commands/status.md
  - fix: 路径正确

- [x] references/dev-doc/ 模板文件与最新格式一致性检查
  - severity: P2
  - files: references/dev-doc/
  - fix: 模板文件已存在且格式正确
