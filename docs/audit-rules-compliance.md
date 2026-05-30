# 项目规则合规审计报告

审计日期：2026-05-31
审计范围：dow CLI 全部 Rust 源码（~10,229 行）+ 项目结构
审计依据：CLAUDE.md 中定义的 6 大类规则

---

## 总评

**整体合规度：良好（85/100）**

项目结构清晰，模块边界合理，错误处理大体规范。主要问题集中在文档可追溯性不完整和少量静默错误处理。

---

## 逐项审计

### 1. 需求与变更边界 ✅ 符合

- 项目通过 hook 机制严格控制写入权限，变更边界清晰
- guard.rs 实现了完整的路径穿越检测、跨分支拦截、阶段性控制
- iterate.rs 预览→确认两阶段执行，符合"最小惊讶原则"

### 2. 文档与长期维护 ⚠️ 部分符合

**符合项：**
- main.rs、cli.rs、core/mod.rs 有文件头结构树注释
- doc_validator.rs、fix.rs 有 Related Docs 链接

**不符合项：**
| 文件 | 问题 |
|------|------|
| `hooks/guard.rs` | 有文件树但无 Related Docs 链接 |
| `hooks/context.rs` | 无 Related Docs |
| `hooks/post_write.rs` | 无 Related Docs |
| `hooks/post_bash.rs` | 无 Related Docs |
| `hooks/save_changelog.rs` | 无 Related Docs |
| `commands/status.rs` | 无 Related Docs |
| `commands/iterate.rs` | 无 Related Docs |
| `commands/scan.rs` | 无 Related Docs |
| `commands/issue.rs` | 无 Related Docs |
| `core/version.rs` | 无 Related Docs |
| `core/yaml.rs` | 无 Related Docs |
| `core/config.rs` | 无 Related Docs |
| `core/github.rs` | 无 Related Docs |
| `core/platform.rs` | 无 Related Docs |
| `core/archive_db.rs` | 无 Related Docs |
| `core/doc_root.rs` | 无 Related Docs |

另外，doc_root.rs 和 yaml.rs 文件头注释路径写的是 `dow/src/lib/` 而非正确的 `dow/src/core/`。

### 3. 实现复杂度控制 ✅ 符合

- **简单优先**：yaml.rs 手写轻量 YAML 解析（仅键值对），未引入 serde_yaml
- **软件杠杆**：合理使用 clap、serde、rusqlite、chrono、reqwest 等成熟库
- **组合优于集成**：模块边界清晰
  - `core/` = 基础设施（yaml、version、config、platform、github、archive_db、doc_root、doc_validator）
  - `commands/` = 子命令实现（每命令一文件）
  - `hooks/` = hook 子命令（每 hook 一文件）
  - 各层通过明确接口协作，无循环依赖

### 4. 接口与模块边界 ✅ 符合

- **高内聚低耦合**：每个 command 文件只处理一个子命令
- **可扩展性**：
  - archive_db 含 schema_version 表，支持未来迁移
  - STATUS.yaml 有明确字段枚举，新增字段有 yaml::set 的 optional_fields 白名单
  - VERSION 文件格式 `(branch)version` 支持多分支并行

### 5. 错误处理与失败策略 ⚠️ 部分符合

**符合项：**
- 统一 DowError 类型，全局 exit code
- status.rs: 非法阶段跳转有清晰错误信息
- iterate.rs: 前置校验完备（任务完成度、P0 issue、文档合法性）
- guard.rs: deny 原因清晰，告知用户怎么解决
- version.rs: validate_semver 快速失败

**不符合项（静默错误/强制降级）：**

| 位置 | 问题 | 规则冲突 |
|------|------|----------|
| `post_write.rs:51` | `yaml::touch_updated(&status_file).ok()` | 静默忽略时间戳更新失败 |
| `post_write.rs:91-93` | `yaml::set(...).ok()` × 3 | audit 模式切换失败被吞 |
| `fix.rs:125,178` | `fs::write(path, &new_content).ok()` | 修复写入失败静默跳过 |
| `validate.rs:186,204,259,263` | 多处 `.ok()` | 目录创建/写入失败被忽略 |
| `archive.rs:270,279` | `fs::remove_dir_all().ok()` | 迁移后删除失败被吞 |
| `iterate.rs:164,184` | `fs::remove_file(entry.path()).ok()` | 归档后删除源文件失败被吞 |
| `output.rs:8` | `serde_json::to_string_pretty(value).unwrap()` | 理论上 Serialize trait 保证安全，可接受 |

**严重度分析**：
- `fix.rs` 和 `iterate.rs` 中的静默失败最严重——用户认为修复/归档成功了，实际文件没变
- hook 中的 `.ok()` 可以理解为"best effort"（hook 不应阻塞主流程），但应至少 `eprintln!` 一下

### 6. 测试约束 ✅ 符合

- 存在独立的集成测试目录 `tests/`（6 个 .rs + 4 个 .sh）
- core 模块有单元测试（yaml、config、github、platform）
- 测试在 `dow/tmp/` 或 `tmp/test_target_project/` 中隔离执行

---

## 额外发现

### A. 文件头路径注释错误

```
// dow/src/lib/
// ├── doc_root.rs
```

实际路径为 `dow/src/core/doc_root.rs`，`yaml.rs` 同理。编译不受影响但文档不准确。

### B. 代码重复

`has_active_work()` 逻辑在以下位置重复实现：
- `guard.rs:490-530` (has_active_work)
- `context.rs:222-240` (count_undone_in_active_tasks)
- `status.rs:302-320` (has_open_tasks)
- `post_write.rs:100-160` (check_task_completion)

四处都是"遍历 task 文件，检查 `- [ ]` 行"。不违反规则，但如果未来变更 task 格式会需要改四处。

### C. guard.rs 的 Bash 命令解析局限

`extract_write_targets_from_command()` 是启发式解析，已知遗漏：
- `install` 命令的目标路径
- `rsync` 的目标
- heredoc 中的重定向

这是安全边界，可接受的权衡（完美解析需要 shell parser），但应在注释中标注"best-effort"。

---

## 补充审计：公共代码提取

### 重复模式统计

| 重复模式 | 出现次数 | 涉及文件数 |
|----------|----------|-----------|
| `fs::read_dir(task_dir)` + 遍历 task 文件 | 17 次 | 10 文件 |
| `fs::read_dir(issue_dir)` + 遍历 issue 文件 | 18 次 | 10 文件 |
| `starts_with("- [x]")` 行匹配 | 19 次 | 8 文件 |
| `starts_with("- [ ]")` 行匹配 | 14 次 | 7 文件 |
| `starts_with("task_") && ends_with(".md")` 文件名过滤 | 13 次 | 8 文件 |
| `starts_with("issue_") && ends_with(".md")` 文件名过滤 | 11 次 | 7 文件 |
| `read_version_info()` (version + tag check) | 2 处独立实现 | context.rs + status.rs |

### 可提取的公共函数

#### 1. task/issue 文件遍历 (最高优先)

```rust
// 建议提取到 core/ 中
pub fn iter_task_files(doc_root: &Path) -> Vec<PathBuf> { ... }
pub fn iter_issue_files(doc_root: &Path) -> Vec<PathBuf> { ... }
pub fn iter_active_task_files(doc_root: &Path) -> Vec<PathBuf> { ... }  // 仅 task_ 前缀
pub fn iter_open_issue_files(doc_root: &Path) -> Vec<PathBuf> { ... }   // 仅 issue_ 前缀
```

当前 10 个文件各自重复：打开目录 → 过滤文件名前缀 → 过滤 `.md` 后缀。如果文件命名规则变化（如新增前缀），需改 10+ 处。

#### 2. checklist 统计 (高优先)

```rust
pub struct ChecklistStats {
    pub total: u32,
    pub done: u32,
}

pub fn count_checklist(content: &str) -> ChecklistStats { ... }
pub fn has_undone_items(content: &str) -> bool { ... }
```

`content.lines().filter(|l| l.starts_with("- [")).count()` 这个表达式出现 19+ 次，散布在 guard、context、status、iterate、post_write、check、fix、doc_validator 中。

#### 3. version + tag 读取

```rust
pub fn read_version_with_tag() -> (String, String) { ... }
```

`status.rs:259` 和 `context.rs:416` 有两个**完全相同**的 `read_version_info()` 函数副本。

#### 4. severity/priority 字段提取

`save_changelog.rs` 和 `context.rs` 都有从 task/issue 行后续子行中提取 severity/priority 并排序的逻辑。模式相同，但不是逐字重复，提取优先级较低。

### 风险分析

**如果不提取会怎样？**

假设未来需要：
- 变更 task 文件命名规则（如 `task_` → `t_`）→ 改 13+ 处
- 变更 checklist 格式（如 `- [x]` → `- [done]`）→ 改 19+ 处
- 新增 issue 文件前缀（如 `wontfix_issue_`）→ 改 11+ 处

这不是"三行重复"的级别——是核心领域逻辑的 10+ 处散落。

### 建议方案

在 `core/` 中新增一个 `task_store.rs`（或扩展现有 `doc_validator.rs`），提供：
- 文件枚举（替代散落的 read_dir + filter）
- checklist 统计（替代散落的 `- [` 行计数）
- read_version_info（消除 status/context 重复）

调用方改为 `task_store::iter_active_files(doc_root)` 等一行调用。

---

## 修复优先级建议

| 优先级 | 问题 | 影响 |
|--------|------|------|
| P0 | fix.rs / iterate.rs 静默写入失败 | 用户以为操作成功但实际没生效 |
| P1 | 文件头路径注释错误（core/ vs lib/） | 文档不可信 |
| P1 | 大量文件缺少 Related Docs 链接 | 违反双向可追溯规则 |
| P2 | hook 中的 `.ok()` 应改为 `eprintln!` | 隐藏潜在问题 |
| P2 | 代码重复（task 检查逻辑） | 维护风险，但当前不紧急 |

---

## 结论

项目在**实现质量**、**模块设计**、**接口边界**、**可扩展性**方面做得好。主要欠缺在**文档可追溯性**（规则 2）和**静默错误处理**（规则 5）。建议优先修复 P0 的静默写入失败问题。
