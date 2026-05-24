# 技术规范（SPEC）

## 1. 概述

本次迭代目标：建立工程版本号的单一真相源，使 git tag 与项目迭代状态自动同步。

核心变更：
1. **VERSION 文件**：项目根目录新增 `VERSION` 文件，作为版本号唯一来源
2. **废弃 `/done` 命令**：交付检查 + 打 tag 职责合并到 `/iterate`
3. **重构 `/iterate`**：交付检查 → 总结 → 归档 → commit & tag → bump 版本，一步完成
4. **废弃 STATUS.yaml iteration 字段**：不再维护该字段，版本信息统一从 VERSION 读取

版本号格式：`a.b.c`
- `a`（major）：大版本更新（架构重构、破坏性变更），agent 检测推荐或用户指定
- `b`（minor）：功能迭代（`/iterate` 默认）或 P0 级 issue 修复
- `c`（patch）：非 P0 issue 修复等小变更

---

## 2. 架构设计

### 系统架构

```
VERSION（项目根目录）
    │
    ├── /iterate → 交付检查 → 归档 → commit → tag v<version> → bump VERSION
    │
    ├── /status → 读取 VERSION → 展示版本 + tag 一致性
    │
    └── inject-context.sh → 读取 VERSION → 注入到上下文输出
```

数据流是单向的：VERSION 文件 → 各消费方读取。只有 `/iterate`（bump）和 `/fix` P0（bump minor）会写入 VERSION。

### 全局约束：分支 ↔ 文档路径一致性

**所有 dev-flow 命令和 hook 必须遵循**：dev-doc 的操作路径必须与当前 git branch 匹配。

- 当存在 `dev-doc/<branch>/STATUS.yaml` 时，DOC_ROOT = `dev-doc/<branch>/`
- 否则 DOC_ROOT = `dev-doc/`
- 任何读写 dev-doc 的操作（命令、hook、归档）都通过 DOC_ROOT 定位，不得硬编码路径
- 如果当前分支与 DOC_ROOT 不一致（如切换分支后 dev-doc 中仍是旧分支的文档），命令应阻断并提示用户

### 模块划分

| 模块 | 职责 | 文件 |
|------|------|------|
| 版本存储 | 持久化版本号 | `VERSION`（项目根） |
| 版本操作库 | bump/read/validate 版本号 | `scripts/lib/version.sh` |
| 迭代命令 | 交付检查 + 归档 + commit & tag + bump | `scripts/commands/iterate.sh`（重构） |
| 上下文注入 | hook 输出中展示版本号 | `scripts/hooks/inject-context.sh`（修改） |
| 状态展示 | `/status` 展示版本信息 | `scripts/commands/status.sh`（修改） |

### 目录结构

```
dev-flow/
├── VERSION                          # 新增：版本号单一真相源
├── scripts/
│   ├── lib/
│   │   └── version.sh              # 新增：版本操作函数库
│   ├── commands/
│   │   ├── iterate.sh             # 重构：交付检查 + 归档 + commit & tag + bump
│   │   ├── status.sh             # 修改：展示版本信息
│   │   ├── check.sh
│   │   └── mode.sh
│   └── hooks/
│       ├── inject-context.sh      # 修改：注入版本号
│       └── ...
├── commands/
│   ├── iterate.md                 # 重写：完整迭代流程（含原 /done 职责）
│   └── status.md                  # 修改：说明版本展示
└── dev-doc/
    └── STATUS.yaml                # 修改：移除 iteration 字段
```

---

## 3. 技术选型

| 领域 | 选择 | 理由 | 备选方案 |
|------|------|------|----------|
| 版本存储格式 | 纯文本 `VERSION` 文件 | `cat VERSION` 即可读取，零依赖，CI/CD 友好 | JSON/YAML（过度设计，增加解析依赖） |
| 版本操作 | Bash 函数库 `version.sh` | 与项目现有技术栈一致（全 bash），无额外运行时依赖 | Python 脚本（引入新语言依赖）、Node semver 包（过重） |
| git tag 执行 | Shell 脚本封装 + AI agent 调用 | 脚本可独立测试和 CI 复用；agent 处理异常（tag 冲突、未 commit 等） | 纯 agent 逻辑（不可测试）、git hook（自动化过度，用户失去控制） |
| 版本校验 | Bash regex 匹配 | `grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'` 足够；项目不需要 semver 预发布标签等复杂语义 | semver 库（功能过剩） |

---

## 4. 数据模型

### 4.1 VERSION 文件

```
2.1.0
```

- 单行纯文本，无换行符后缀（或仅含一个 `\n`）
- 格式：`<major>.<minor>.<patch>`，三段均为非负整数
- 不含 `v` 前缀（tag 加前缀 `v` 是 git tag 的命名规范，不是版本号本身）

### 4.2 STATUS.yaml 变更

```yaml
# 移除前
name: dev-flow
phase: DEV
mode: mvp
iteration: 3          # 将被移除
updated: 2026-05-24 17:19
started: 2026-05-24 17:19

# 移除后
name: dev-flow
phase: DEV
mode: mvp
updated: 2026-05-24 17:19
started: 2026-05-24 17:19
```

`iteration` 字段废弃。所有需要版本号的场景直接读取 `VERSION` 文件。

### 4.3 Git Tag 命名

- 格式：`v<VERSION 文件内容>`，如 `v2.1.0`
- annotated tag（`git tag -a`），message 为 `Release v<version>`
- 仅在 `/iterate` 的 commit & tag 阶段创建

---

## 5. 接口设计

### 5.1 scripts/lib/version.sh — 版本操作函数库

```bash
#!/bin/bash
# 版本操作函数库
# 用法：source scripts/lib/version.sh

# 读取当前版本号
# 返回：版本字符串（如 "2.1.0"），失败返回空字符串
version_read() {
  local version_file="${1:-VERSION}"
  if [ -f "$version_file" ]; then
    cat "$version_file" | tr -d '[:space:]'
  fi
}

# 校验版本号格式
# 参数：$1 = 版本字符串
# 返回：0=合法，1=非法
version_validate() {
  echo "$1" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'
}

# Bump 版本号
# 参数：$1 = 当前版本，$2 = bump 类型（major|minor|patch）
# 输出：新版本号字符串
version_bump() {
  local version="$1"
  local type="${2:-minor}"
  
  local major minor patch
  IFS='.' read -r major minor patch <<< "$version"
  
  case "$type" in
    major) major=$((major + 1)); minor=0; patch=0 ;;
    minor) minor=$((minor + 1)); patch=0 ;;
    patch) patch=$((patch + 1)) ;;
    *) echo "ERROR: unknown bump type: $type" >&2; return 1 ;;
  esac
  
  echo "${major}.${minor}.${patch}"
}

# 写入版本号到文件
# 参数：$1 = 新版本号，$2 = 版本文件路径（默认 VERSION）
version_write() {
  local new_version="$1"
  local version_file="${2:-VERSION}"
  
  if ! version_validate "$new_version"; then
    echo "ERROR: invalid version format: $new_version" >&2
    return 1
  fi
  
  echo "$new_version" > "$version_file"
}

# 检查 git tag 是否存在
# 参数：$1 = 版本号
# 返回：0=存在，1=不存在
version_tag_exists() {
  git tag -l "v$1" | grep -q "v$1"
}

# 创建 git tag
# 参数：$1 = 版本号
# 返回：0=成功，1=失败
version_create_tag() {
  local version="$1"
  
  if version_tag_exists "$version"; then
    echo "ERROR: tag v$version already exists" >&2
    return 1
  fi
  
  git tag -a "v$version" -m "Release v$version"
}
```

### 5.2 /iterate 完整流程（scripts/commands/iterate.sh 重构）

```bash
#!/bin/bash
# /iterate：交付检查 → 总结 → 归档 → commit & tag → bump

source "$(dirname "$0")/../lib/version.sh"

TOPIC="$1"
BUMP_TYPE="${2:-minor}"  # agent 传入：minor（默认）| major | patch

# ===== 阶段 1：交付检查（阻断） =====
# 检查 task 全部完成
# 检查无未关闭 P0 issue
# 任一不通过 → exit 1 报错

# ===== 阶段 2：读取当前版本 =====
VERSION=$(version_read)
if [ -z "$VERSION" ]; then
  echo "[dev-flow] ERROR: VERSION 文件不存在或为空"
  exit 1
fi
if ! version_validate "$VERSION"; then
  echo "[dev-flow] ERROR: VERSION 文件格式非法: $VERSION"
  exit 1
fi

# ===== 阶段 3：归档（用当前版本号命名） =====
ARCHIVE_DIR="dev-doc/archive/v${VERSION}-${TOPIC}"
# 移动 done_task_*、closed_issue_*、CHANGELOG.md
# 复制 SPEC.md、TEST.md 等主文档

# ===== 阶段 4：向用户展示变更摘要，等待确认 =====
# 展示：归档内容、当前版本、将要打的 tag、bump 后的新版本
# 用户确认后继续，否则停止

# ===== 阶段 5：commit & tag =====
# git add 所有变更（包括代码、文档、归档）
# git commit -m "Release v${VERSION}: ${TOPIC}"
# git tag -a "v${VERSION}" -m "Release v${VERSION}"

# ===== 阶段 6：bump 版本号，开启新迭代 =====
NEW_VERSION=$(version_bump "$VERSION" "$BUMP_TYPE")
version_write "$NEW_VERSION"
# 重置 STATUS.yaml phase
# git add VERSION dev-doc/STATUS.yaml
# git commit -m "Start v${NEW_VERSION} iteration"

echo "[dev-flow] 迭代完成：v${VERSION} → v${NEW_VERSION}"
```

**bump 类型决策逻辑（由 agent 在调用脚本前判断）**：
1. 默认 minor
2. 用户显式传参 `--major` → major
3. Agent 检测到架构重构/破坏性变更 → 推荐 major，询问用户确认

### 5.4 inject-context.sh 版本注入

```bash
# 在基础状态输出行中添加版本号：

if [ -f "VERSION" ]; then
  VER=$(cat VERSION | tr -d '[:space:]')
  TAG_EXISTS=$(git tag -l "v$VER" | grep -q "v$VER" && echo "synced" || echo "no-tag")
  echo "[dev-flow ${MODE:-?}] v$VER($TAG_EXISTS) | STAGE: $PHASE | TASK: $DONE/$TOTAL | ISSUE: $OPEN_ISSUES"
else
  echo "[dev-flow ${MODE:-?}] STAGE: $PHASE | TASK: $DONE/$TOTAL | ISSUE: $OPEN_ISSUES"
fi
```

### 5.5 /status 版本展示

```bash
# status.sh 新增版本信息段：

if [ -f "VERSION" ]; then
  VER=$(cat VERSION | tr -d '[:space:]')
  TAG_STATUS="未同步"
  if git tag -l "v$VER" | grep -q "v$VER"; then
    TAG_STATUS="已同步"
  fi
  echo "当前版本：v$VER（git tag: $TAG_STATUS）"
else
  echo "当前版本：未设置（缺少 VERSION 文件）"
fi
```

---

## 6. 开发实践

### 编码规范

- Shell 脚本遵循项目已有风格：bash 4.x 兼容，`set -e` 不强制（因为现有脚本均未使用）
- 函数库使用 `local` 声明局部变量，避免全局污染
- 所有用户可见输出以 `[dev-flow]` 前缀开头
- 错误输出到 stderr（`>&2`）

### Git 工作流

- 本次变更在单次迭代内完成
- VERSION 文件本身纳入 git 追踪
- 首次创建 VERSION 时内容为 `2.2.0`（当前项目已交付 v2.1，本次迭代为新功能）

### 依赖管理

- 无新外部依赖引入
- `version.sh` 作为内部函数库，被其他脚本 `source` 引用
- 要求 git 2.x+（annotated tag 功能）

---

## 7. 非功能需求

### 性能

- `version_read()` 执行时间 < 5ms（单次 cat + tr）
- `inject-context.sh` 新增 git tag 检查约 10-20ms（`git tag -l` 在本地仓库是 O(n) 扫描，但 tag 数量极少）
- 不引入性能瓶颈

### 安全

- VERSION 文件为纯文本，不含可执行内容
- `version.sh` 中所有外部输入经过 regex 校验后才使用
- git tag 操作仅在明确的用户流程中触发（不在自动 hook 中执行）

### 兼容性

- Linux/WSL：完全支持（主要运行环境）
- macOS：`sed -i` 不涉及（VERSION 文件操作使用 `echo >` 覆盖写入）
- 向后兼容：旧项目无 VERSION 文件时，相关逻辑静默跳过（不阻断）

---

## 8. 风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| git tag 已存在（重复 /iterate） | tag 创建失败 | 中 | `version_tag_exists` 前置检查，已存在则跳过并提示 |
| 用户确认后 commit 失败 | 归档状态不一致 | 低 | commit 前 `git status --porcelain` 检查；失败时提示手动处理 |
| VERSION 文件被手动篡改 | 格式非法导致脚本报错 | 低 | `version_validate` 校验，非法时报错提示修复 |
| agent 误判 major bump | 版本号跳跃过大 | 低 | major 推荐需用户确认，用户有最终决定权 |
| STATUS.yaml 中 iteration 字段被旧脚本读取 | 归档路径命名问题 | 中 | iterate.sh 中归档目录改为读取 VERSION；逐步移除 iteration 读取逻辑 |
| 多人协作 tag 冲突 | 同版本号不同 commit | 低 | 项目当前为单人工作流；如未来需要，可加 `git fetch --tags` 前置检查 |

---

## 9. 待定事项

1. **初始 VERSION 文件创建**：本次迭代创建 `VERSION` 文件内容为 `2.2.0`。此操作在 TASK 阶段的第一个任务中完成。

2. **P0 issue 修复触发 minor bump 的时机**：PRD 约定 P0 issue 修复动 minor。建议在 `/fix` 关闭 P0 issue 后，由 agent 自动 bump minor 并写入 VERSION。具体实现待 TASK 阶段细化。

3. **废弃 `/done` 命令的清理工作**：

   **注意**：`DONE` 作为 STATUS.yaml 的 phase 值保留（表示"测试通过，可 iterate"），但不再由独立命令触发，改由 `/test` 通过后自动设置。

   涉及文件及变更：

   **废弃 `/done` 命令：**
   - 删除 `commands/done.md`
   - 修改 `commands/iterate.md`：移除"自动触发 /done 检查"逻辑，改为内置交付检查
   - 修改 `commands/mode.md`：命令表移除 `/done` 行；流程图中 `→ done` 改为 `→ iterate`
   - 修改 `commands/test.md`：全部通过后提示改为 `/iterate` 而非 `/done`
   - 修改 `CLAUDE.md`：命令表移除 `/done`，流程图去掉 `/done` 节点
   - 修改 `AGENTS.md`：命令表移除 `/done`
   - 修改 `README.md` 和 `README.zh-CN.md`：命令列表和表格移除 `/done`
   - 修改 `skills/dev-flow/SKILL.md`、`.claude/skills/dev-flow/SKILL.md` 和 `.agents/skills/dev-flow/SKILL.md`：description 和命令列表移除 `/done`
   - 修改 `.claude-plugin/plugin.json`：commands 列表移除 `./commands/done.md`
   - 修改 `.claude-plugin/marketplace.json`：description 移除 `/done`
   - 修改 `scripts/commands/iterate.sh`：移除注释"前置依赖：/done 检查应已通过"，内置交付检查逻辑
   - 修改 `scripts/commands/status.sh`：`TEST` 阶段的 NEXT 从 `/done` 改为 `/iterate`
   - 修改 `scripts/commands/mode.sh`：流程描述中 `→ DONE` 改为 `→ ITERATE`
   - 修改 `scripts/hooks/inject-context.sh`：TEST HINTS 中 `/done` 改为 `/iterate`
   - 修改 `dev-doc/PRD.md`：更新为最终设计（`/done` 职责并入 `/iterate`）

   **废弃 STATUS.yaml `iteration` 字段（改用 VERSION 文件）：**
   - 修改 `commands/init.md`：STATUS.yaml 模板移除 `iteration: 1`；注释中去掉 iteration 引用
   - 修改 `scripts/commands/iterate.sh`：不再读写 `iteration` 字段，改为读写 VERSION 文件
   - 修改 `scripts/commands/status.sh`：不再读取 `iteration`，改为读取 VERSION
   - 修改 `scripts/commands/mode.sh`：STATUS.yaml 模板移除 `iteration: 1`
   - 修改 `scripts/init/validate.sh`：必需字段列表移除 `iteration`
   - 修改 `references/dev-doc/STATUS.yaml`：移除 iteration 字段及注释
   - 修改 `references/dev-flow-spec.md`：迭代版本号来源改为 VERSION 文件
   - 修改 `dev-doc/STATUS.yaml`：移除当前的 `iteration: 3` 字段
   - 修改测试文件（`tests/test_*.sh`）：所有 STATUS.yaml fixture 中移除 `iteration` 字段，`test_commands.sh` 中 iterate 递增断言改为验证 VERSION 文件

   **不修改（归档文件为历史快照）：**
   - `dev-doc/archive/` 下所有文件保持原样
   - `dev-doc/BRAINSTORM.md` 保持原样（历史讨论记录）
