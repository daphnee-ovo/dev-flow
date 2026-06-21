// dow/src/hooks/
// ├── post_write.rs  -- dow hooks post-write（写后联动）
//    合并：update-status / audit 触发 / 任务完成度 / done_/closed_ 重命名 / 同步提醒
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::fs;
use std::io::Read as IoRead;
use std::path::Path;

pub fn run(file: Option<String>, codex_hook: bool, _kiro_hook: bool) -> Result<i32, DowError> {
    // 从命令行参数、stdin hook JSON、或环境变量获取文件路径
    let changed_file = file
        .or_else(|| read_file_path_from_stdin())
        .unwrap_or_default();

    if changed_file.is_empty() || !Path::new(crate::core::DOC_DIR).is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    // 分支校验：写入 .dev-doc/ 内文件时检查是否属于当前分支
    if changed_file.starts_with(".dev-doc/") {
        if let Some(branch) = doc_root::current_branch() {
            let expected_prefix = format!(".dev-doc/{}/", branch);
            let normalized = changed_file.replace('\\', "/");
            // 只对有分支子目录格式的路径做校验
            if !normalized.starts_with(&expected_prefix)
                && !normalized.starts_with(".dev-doc/archive/")
            {
                let rest = &normalized[".dev-doc/".len()..];
                if let Some(target_branch) = rest.split('/').next() {
                    if Path::new(&format!(".dev-doc/{}", target_branch)).is_dir()
                        && target_branch != "archive"
                    {
                        emit_message(
                            codex_hook,
                            format!(
                            "[dev-flow] ⚠ 写入了其他分支的文件：{}（当前分支：{}）",
                            changed_file, branch
                            ),
                        );
                        emit_message(
                            codex_hook,
                            "→ 这可能是误操作，guard 应已在 PreToolUse 阶段拦截。".to_string(),
                        );
                    }
                }
            }
        }
    }

    // 1. 更新时间戳（.dev-doc 内文件变更时）
    if changed_file.starts_with(".dev-doc/") && status_file.exists() {
        let is_status = changed_file == status_file.to_string_lossy();
        let is_changelog = changed_file.ends_with("CHANGELOG.md");
        if !is_status && !is_changelog {
            if let Err(e) = yaml::touch_updated(&status_file) {
                emit_warning(format!(
                    "[dow] 警告: 更新时间戳失败 ({}): {}",
                    status_file.display(),
                    e
                ));
            }
        }
    }

    if !status_file.exists() {
        return Ok(0);
    }

    let mut phase = yaml::get(&status_file, "phase")
        .ok()
        .flatten()
        .unwrap_or_default();
    let mode = yaml::get(&status_file, "mode")
        .ok()
        .flatten()
        .unwrap_or_default();

    // 1.5 audit 模式自动触发
    if changed_file.contains("/issue/issue_") && !mode.starts_with("audit/") && phase != "DEV" {
        enter_audit_mode(&status_file, &mode, codex_hook);
        // enter_audit_mode 已将 phase 设为 DEV，刷新本地变量
        phase = "DEV".to_string();
    }

    // 2. 任务完成度检测（仅 DEV 阶段）
    if phase == "DEV" {
        check_task_completion(&doc_root_path, &status_file, codex_hook);
        check_issue_completion(&doc_root_path, &status_file, &mode, codex_hook);
    }

    // 3. 代码变更同步提醒
    if phase == "DEV" && !changed_file.starts_with(".dev-doc/") {
        check_code_sync(&changed_file, &doc_root_path, &mode, codex_hook);
        check_persistent_docs_reminder(&changed_file, &status_file, codex_hook);
    }

    Ok(0)
}

fn enter_audit_mode(status_file: &Path, current_mode: &str, codex_hook: bool) {
    let new_mode = format!("audit/{}", current_mode);
    if let Err(e) = yaml::set(status_file, "mode", &new_mode) {
        emit_warning(format!("[dow] 警告: 设置 audit 模式失败: {}", e));
    }
    if let Err(e) = yaml::set(status_file, "phase", "DEV") {
        emit_warning(format!("[dow] 警告: 设置 phase=DEV 失败: {}", e));
    }
    if let Err(e) = yaml::touch_updated(status_file) {
        emit_warning(format!("[dow] 警告: 更新 audit 模式时间戳失败: {}", e));
    }
    emit_message(
        codex_hook,
        format!(
        "[dev-flow] 检测到审计 issue，自动进入 audit 模式（原模式：{}）",
        current_mode
        ),
    );
}

fn check_task_completion(
    doc_root: &Path,
    status_file: &Path,
    codex_hook: bool,
) {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return;
    }

    let (done, total) = crate::core::task_store::count_all_checklist(&task_dir);

    let entries: Vec<_> = fs::read_dir(&task_dir)
        .map(|e| e.flatten().collect())
        .unwrap_or_default();

    if total == 0 {
        return;
    }

    if done == total {
        emit_message(
            codex_hook,
            format!("[dev-flow] 所有任务已完成（{}/{}）。", done, total),
        );
        emit_message(
            codex_hook,
            "→ 立即执行 /test 进行全量验证。".to_string(),
        );
    } else {
        // 检查 exec_mode
        let exec_mode = yaml::get(status_file, "exec_mode")
            .ok()
            .flatten()
            .unwrap_or_else(|| "step".to_string());

        // 找最近完成的任务
        for entry in &entries {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("task_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Some(last_done) = content.lines().filter(|l| l.starts_with("- [x]")).last() {
                    let task_name = last_done.trim_start_matches("- [x] ");
                    emit_message(
                        codex_hook,
                        format!("[dev-flow] 任务完成（{}/{}）：{}", done, total, task_name),
                    );
                    if exec_mode == "continuous" {
                        emit_message(
                            codex_hook,
                            "→ [continuous] 自动推进：执行 /devtest 并继续下一个任务。"
                                .to_string(),
                        );
                    } else {
                        emit_message(
                            codex_hook,
                            "→ 自动触发 /devtest。立即对该任务执行例行测试，不需要询问用户。"
                                .to_string(),
                        );
                    }
                    break;
                }
            }
        }
    }

    // done_ 前缀自动重命名
    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("task_") || !name.ends_with(".md") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let file_total = content.lines().filter(|l| l.starts_with("- [")).count();
            let file_done = content.lines().filter(|l| l.starts_with("- [x]")).count();
            if file_total > 0 && file_total == file_done {
                let new_name = format!("done_{}", name);
                let new_path = task_dir.join(&new_name);
                if !new_path.exists() {
                    if let Err(e) = fs::rename(entry.path(), &new_path) {
                        emit_warning(format!(
                            "[dow] 警告: 重命名任务文件为 done_ 失败 ({}): {}",
                            name, e
                        ));
                    } else {
                        emit_message(
                            codex_hook,
                            format!("[dev-flow] 批次全部完成，已标记：{}", new_name),
                        );
                    }
                }
            }
        }
    }
}

fn check_issue_completion(
    doc_root: &Path,
    status_file: &Path,
    mode: &str,
    codex_hook: bool,
) {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        if mode.starts_with("audit/") {
            exit_audit_mode(status_file, mode, codex_hook);
        }
        return;
    }

    let mut has_open_issue = false;
    if let Ok(issue_entries) = fs::read_dir(&issue_dir) {
        for entry in issue_entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("issue_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let file_total = content.lines().filter(|l| l.starts_with("- [")).count();
                let file_done = content.lines().filter(|l| l.starts_with("- [x]")).count();
                if file_total > 0 && file_total == file_done {
                    let new_name = format!("closed_{}", name);
                    let new_path = issue_dir.join(&new_name);
                    if !new_path.exists() {
                        if let Err(e) = fs::rename(entry.path(), &new_path) {
                            emit_warning(format!(
                                "[dow] 警告: 重命名 issue 文件为 closed_ 失败 ({}): {}",
                                name, e
                            ));
                        } else {
                            emit_message(
                                codex_hook,
                                format!("[dev-flow] Issue 全部关闭：{}", new_name),
                            );
                        }
                    }
                } else {
                    has_open_issue = true;
                }
            }
        }
    }

    // audit 模式下无 open issue → 自动退出 audit
    if !has_open_issue && mode.starts_with("audit/") {
        exit_audit_mode(status_file, mode, codex_hook);
    }
}

fn exit_audit_mode(status_file: &Path, mode: &str, codex_hook: bool) {
    let original_mode = mode.strip_prefix("audit/").unwrap_or("fast");
    if let Err(e) = yaml::set(status_file, "mode", original_mode) {
        emit_warning(format!("[dow] 警告: 退出 audit 模式失败: {}", e));
    }
    if let Err(e) = yaml::touch_updated(status_file) {
        emit_warning(format!("[dow] 警告: 更新时间戳失败: {}", e));
    }
    emit_message(
        codex_hook,
        format!(
        "[dev-flow] 所有 issue 已关闭，自动退出 audit 模式（恢复为：{}）",
        original_mode
        ),
    );
}

/// 从 stdin 读取 Claude Code hook JSON，提取 tool_input.file_path
fn read_file_path_from_stdin() -> Option<String> {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() || buf.is_empty() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(&buf).ok()?;
    json.get("tool_input")
        .and_then(|ti| ti.get("file_path"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn check_persistent_docs_reminder(
    changed_file: &str,
    status_file: &Path,
    codex_hook: bool,
) {
    // 排除 docs/ 自身的变更
    if changed_file.starts_with("docs/") || changed_file == "README.md" {
        return;
    }

    let docs = yaml::get_list(status_file, "docs").unwrap_or_default();
    if docs.is_empty() {
        return;
    }

    // 统计 unstaged + staged 变更文件数（排除 .dev-doc/ 和 docs/）
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output();

    let changed_count = match output {
        Ok(o) => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.starts_with(".dev-doc/") && !l.starts_with("docs/") && *l != "README.md")
                .count()
        }
        Err(_) => return,
    };

    // 阈值：3 个文件
    if changed_count >= 3 {
        emit_message(
            codex_hook,
            format!(
            "[dev-flow] 提示：代码变更较大（{}个文件），请检查是否需要更新持久化文档",
            changed_count
            ),
        );
        emit_message(
            codex_hook,
            format!("  注册文档：{}", docs.join(", ")),
        );
    }
}

fn check_code_sync(
    changed_file: &str,
    doc_root: &Path,
    mode: &str,
    codex_hook: bool,
) {
    if mode == "fast" {
        return;
    }
    let code_exts = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".rs", ".go", ".java", ".rb", ".php", ".vue",
        ".svelte",
    ];
    if !code_exts.iter().any(|ext| changed_file.ends_with(ext)) {
        return;
    }

    let spec_file = doc_root.join("SPEC.md");
    if !spec_file.exists() {
        return;
    }

    // 从文件名提取模块名
    let basename = Path::new(changed_file)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if let Ok(spec_content) = fs::read_to_string(&spec_file) {
        if spec_content
            .to_lowercase()
            .contains(&basename.to_lowercase())
        {
            emit_message(
                codex_hook,
                format!(
                    "[dev-flow] 代码文件 {} 已修改，SPEC.md 中有该模块的描述。",
                    changed_file
                ),
            );
            emit_message(
                codex_hook,
                "→ 如果修改了 API 接口/数据结构/目录组织，必须同步更新 SPEC.md。".to_string(),
            );
        }
    }
}

fn emit_message(codex_hook: bool, message: String) {
    if !codex_hook {
        println!("{}", message);
    }
}

fn emit_warning(message: String) {
    eprintln!("{}", message);
}
