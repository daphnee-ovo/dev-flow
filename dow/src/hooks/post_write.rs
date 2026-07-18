// dow/src/hooks/
// ├── post_write.rs  -- dow hooks post-write (post-write hooks)
//    Merged: update-status / audit trigger / task completion / done_/closed_ rename / sync reminder
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
    // Get file path from CLI arg, stdin hook JSON, or environment variable
    let changed_file = file
        .or_else(|| read_file_path_from_stdin())
        .unwrap_or_default();

    if changed_file.is_empty() || !Path::new(crate::core::DOC_DIR).is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    // Branch validation: check if file written under .dev-doc/ belongs to current branch
    if changed_file.starts_with(".dev-doc/") {
        if let Some(branch) = doc_root::current_branch() {
            let expected_prefix = format!(".dev-doc/{}/", branch);
            let normalized = changed_file.replace('\\', "/");
            // Only validate paths with branch subdirectory format
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
                            "[dev-flow] ⚠ Wrote to another branch's file: {} (current branch: {})",
                            changed_file, branch
                            ),
                        );
                        emit_message(
                            codex_hook,
                            "→ This may be an error, guard should have blocked it in PreToolUse phase.".to_string(),
                        );
                    }
                }
            }
        }
    }

    // 1. Update timestamp (when .dev-doc files change)
    if changed_file.starts_with(".dev-doc/") && status_file.exists() {
        let is_status = changed_file == status_file.to_string_lossy();
        let is_changelog = changed_file.ends_with("CHANGELOG.md");
        if !is_status && !is_changelog {
            if let Err(e) = yaml::touch_updated(&status_file) {
                emit_warning(format!(
                    "[dow] Warning: failed to update timestamp ({}): {}",
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

    // 1.5 Auto-trigger audit mode
    if changed_file.contains("/issue/issue_") && !mode.starts_with("audit/") && phase != "DEV" {
        enter_audit_mode(&status_file, &mode, codex_hook);
        // enter_audit_mode already set phase to DEV, refresh local variable
        phase = "DEV".to_string();
    }

    // 2. Task completion detection (DEV phase only)
    if phase == "DEV" {
        check_task_completion(&doc_root_path, &status_file, codex_hook);
        check_issue_completion(&doc_root_path, &status_file, &mode, codex_hook);
    }

    // 3. Code change sync reminder
    if phase == "DEV" && !changed_file.starts_with(".dev-doc/") {
        check_code_sync(&changed_file, &doc_root_path, &mode, codex_hook);
        check_persistent_docs_reminder(&changed_file, &status_file, codex_hook);
    }

    Ok(0)
}

fn enter_audit_mode(status_file: &Path, current_mode: &str, codex_hook: bool) {
    let new_mode = format!("audit/{}", current_mode);
    if let Err(e) = yaml::set(status_file, "mode", &new_mode) {
        emit_warning(format!("[dow] Warning: failed to set audit mode: {}", e));
    }
    if let Err(e) = yaml::set(status_file, "phase", "DEV") {
        emit_warning(format!("[dow] Warning: failed to set phase=DEV: {}", e));
    }
    if let Err(e) = yaml::touch_updated(status_file) {
        emit_warning(format!(
            "[dow] Warning: failed to update audit mode timestamp: {}",
            e
        ));
    }
    emit_message(
        codex_hook,
        format!(
        "[dev-flow] Audit issue detected, automatically entering audit mode (original mode: {})",
        current_mode
        ),
    );
}

fn check_task_completion(doc_root: &Path, status_file: &Path, codex_hook: bool) {
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
            format!("[dev-flow] All tasks completed ({}/{}).", done, total),
        );
        emit_message(
            codex_hook,
            "→ Immediately run /test for full validation.".to_string(),
        );
    } else {
        // Check exec_mode
        let exec_mode = yaml::get(status_file, "exec_mode")
            .ok()
            .flatten()
            .unwrap_or_else(|| "step".to_string());

        // Find most recently completed task
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
                        format!(
                            "[dev-flow] Task completed ({}/{}): {}",
                            done, total, task_name
                        ),
                    );
                    if exec_mode == "continuous" {
                        emit_message(
                            codex_hook,
                            "→ [continuous] Auto-advance: run /devtest and continue to next task."
                                .to_string(),
                        );
                    } else {
                        emit_message(
                            codex_hook,
                            "→ Auto-trigger /devtest. Immediately run routine tests for this task, no need to ask user."
                                .to_string(),
                        );
                    }
                    break;
                }
            }
        }
    }

    // Auto-rename with done_ prefix
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
                            "[dow] Warning: failed to rename task file to done_ ({}): {}",
                            name, e
                        ));
                    } else {
                        emit_message(
                            codex_hook,
                            format!("[dev-flow] Batch fully completed, marked: {}", new_name),
                        );
                    }
                }
            }
        }
    }
}

fn check_issue_completion(doc_root: &Path, status_file: &Path, mode: &str, codex_hook: bool) {
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
                                "[dow] Warning: failed to rename issue file to closed_ ({}): {}",
                                name, e
                            ));
                        } else {
                            emit_message(
                                codex_hook,
                                format!("[dev-flow] All issues closed: {}", new_name),
                            );
                        }
                    }
                } else {
                    has_open_issue = true;
                }
            }
        }
    }

    // In audit mode with no open issues → auto-exit audit
    if !has_open_issue && mode.starts_with("audit/") {
        exit_audit_mode(status_file, mode, codex_hook);
    }
}

fn exit_audit_mode(status_file: &Path, mode: &str, codex_hook: bool) {
    let original_mode = mode.strip_prefix("audit/").unwrap_or("fast");
    if let Err(e) = yaml::set(status_file, "mode", original_mode) {
        emit_warning(format!("[dow] Warning: failed to exit audit mode: {}", e));
    }
    if let Err(e) = yaml::touch_updated(status_file) {
        emit_warning(format!("[dow] Warning: failed to update timestamp: {}", e));
    }
    emit_message(
        codex_hook,
        format!(
        "[dev-flow] All issues closed, automatically exiting audit mode (restoring to: {})",
        original_mode
        ),
    );
}

/// Read Claude Code hook JSON from stdin, extract tool_input.file_path
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

fn check_persistent_docs_reminder(changed_file: &str, status_file: &Path, codex_hook: bool) {
    // Exclude changes to docs/ itself
    if changed_file.starts_with("docs/") || changed_file == "README.md" {
        return;
    }

    let docs = yaml::get_list(status_file, "docs").unwrap_or_default();
    if docs.is_empty() {
        return;
    }

    // Count unstaged + staged changed files (excluding .dev-doc/ and docs/)
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output();

    let changed_count = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.starts_with(".dev-doc/") && !l.starts_with("docs/") && *l != "README.md")
            .count(),
        Err(_) => return,
    };

    // Threshold: 3 files
    if changed_count >= 3 {
        emit_message(
            codex_hook,
            format!(
            "[dev-flow] Reminder: significant code changes ({} files), please check if persistent docs need updating",
            changed_count
            ),
        );
        emit_message(
            codex_hook,
            format!("  Registered docs: {}", docs.join(", ")),
        );
    }
}

fn check_code_sync(changed_file: &str, doc_root: &Path, mode: &str, codex_hook: bool) {
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

    // Extract module name from filename
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
                    "[dev-flow] Code file {} modified, module described in SPEC.md.",
                    changed_file
                ),
            );
            emit_message(
                codex_hook,
                "→ If API interfaces/data structures/directory organization changed, SPEC.md must be updated synchronously.".to_string(),
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
