// dow/src/hooks/
// ├── post_write.rs  -- dow hooks post-write（写后联动）
//    合并：update-status / audit 触发 / 任务完成度 / done_/closed_ 重命名 / 同步提醒

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::fs;
use std::io::Read as IoRead;
use std::path::Path;

pub fn run(file: Option<String>) -> Result<i32, DowError> {
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
            if !normalized.starts_with(&expected_prefix) && !normalized.starts_with(".dev-doc/archive/") {
                let rest = &normalized[".dev-doc/".len()..];
                if let Some(target_branch) = rest.split('/').next() {
                    if Path::new(&format!(".dev-doc/{}", target_branch)).is_dir() && target_branch != "archive" {
                        println!(
                            "[dev-flow] ⚠ 写入了其他分支的文件：{}（当前分支：{}）",
                            changed_file, branch
                        );
                        println!("→ 这可能是误操作，guard 应已在 PreToolUse 阶段拦截。");
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
            yaml::touch_updated(&status_file).ok();
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
        enter_audit_mode(&status_file, &mode);
        // enter_audit_mode 已将 phase 设为 DEV，刷新本地变量
        phase = "DEV".to_string();
    }

    // 2. 任务完成度检测（仅 DEV 阶段）
    if phase == "DEV" {
        check_task_completion(&doc_root_path, &status_file);
        check_issue_completion(&doc_root_path);
    }

    // 3. 代码变更同步提醒
    if phase == "DEV" && !changed_file.starts_with(".dev-doc/") {
        check_code_sync(&changed_file, &doc_root_path, &mode);
    }

    Ok(0)
}

fn enter_audit_mode(status_file: &Path, current_mode: &str) {
    let new_mode = format!("audit/{}", current_mode);
    yaml::set(status_file, "mode", &new_mode).ok();
    yaml::set(status_file, "phase", "DEV").ok();
    yaml::touch_updated(status_file).ok();
    println!(
        "[dev-flow] 检测到审计 issue，自动进入 audit 模式（原模式：{}）",
        current_mode
    );
}

fn check_task_completion(doc_root: &Path, status_file: &Path) {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return;
    }

    let mut total = 0u32;
    let mut done = 0u32;

    let entries: Vec<_> = fs::read_dir(&task_dir)
        .map(|e| e.flatten().collect())
        .unwrap_or_default();

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("task_") || !name.ends_with(".md") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(entry.path()) {
            total += content.lines().filter(|l| l.starts_with("- [")).count() as u32;
            done += content.lines().filter(|l| l.starts_with("- [x]")).count() as u32;
        }
    }

    if total == 0 {
        return;
    }

    if done == total {
        println!("[dev-flow] 所有任务已完成（{}/{}）。", done, total);
        println!("→ 立即执行 /test 进行全量验证。");
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
                if let Some(last_done) = content
                    .lines()
                    .filter(|l| l.starts_with("- [x]"))
                    .last()
                {
                    let task_name = last_done.trim_start_matches("- [x] ");
                    println!("[dev-flow] 任务完成（{}/{}）：{}", done, total, task_name);
                    if exec_mode == "continuous" {
                        println!("→ [continuous] 自动推进：执行 /devtest 并继续下一个任务。");
                    } else {
                        println!("→ 自动触发 /devtest。立即对该任务执行例行测试，不需要询问用户。");
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
                    fs::rename(entry.path(), &new_path).ok();
                    println!("[dev-flow] 批次全部完成，已标记：{}", new_name);
                }
            }
        }
    }

}

fn check_issue_completion(doc_root: &Path) {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return;
    }
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
                        fs::rename(entry.path(), &new_path).ok();
                        println!("[dev-flow] Issue 全部关闭：{}", new_name);
                    }
                }
            }
        }
    }
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

fn check_code_sync(changed_file: &str, doc_root: &Path, mode: &str) {
    if mode == "fast" {
        return;
    }
    let code_exts = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".rs", ".go", ".java",
        ".rb", ".php", ".vue", ".svelte",
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
        if spec_content.to_lowercase().contains(&basename.to_lowercase()) {
            println!(
                "[dev-flow] 代码文件 {} 已修改，SPEC.md 中有该模块的描述。",
                changed_file
            );
            println!("→ 如果修改了 API 接口/数据结构/目录组织，必须同步更新 SPEC.md。");
        }
    }
}
