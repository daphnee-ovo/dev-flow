// dow/src/commands/
// ├── validate.rs  -- dow validate（校验 dev-doc 目录结构与文件规范）

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ValidateOutput {
    doc_root: String,
    auto_fixed: Vec<String>,
    needs_confirm: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve("dev-doc");
    let mut result = ValidateOutput {
        doc_root: doc_root_path.to_string_lossy().to_string(),
        auto_fixed: Vec::new(),
        needs_confirm: Vec::new(),
        warnings: Vec::new(),
    };

    // 1. 目录结构校验
    check_directories(&doc_root_path, &mut result);

    // 2. STATUS.yaml 校验
    check_status_yaml(&doc_root_path, &mut result);

    // 3. task/ 目录校验
    check_task_files(&doc_root_path, &mut result);

    // 4. issue/ 目录校验
    check_issue_files(&doc_root_path, &mut result);

    // 5. CHANGELOG 校验
    check_changelog(&doc_root_path, &mut result);

    // 6. .gitignore 检查
    check_gitignore(&mut result);

    // 7. 根级残留文件检查
    check_stale_root_files(&doc_root_path, &mut result);

    let has_problems = !result.needs_confirm.is_empty() || !result.warnings.is_empty();

    if human {
        print_human(&result);
    } else {
        output::print_json(&result);
    }

    Ok(if has_problems { 1 } else { 0 })
}

fn check_directories(doc_root: &Path, result: &mut ValidateOutput) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp"
    } else {
        "tmp"
    };

    let dirs = [
        doc_root.join("issue"),
        doc_root.join("task"),
        doc_root.join("archive"),
        PathBuf::from("tests"),
        PathBuf::from(project_temp),
    ];

    for dir in &dirs {
        if !dir.exists() {
            fs::create_dir_all(dir).ok();
            result.auto_fixed.push(format!("created_dir:{}", dir.display()));
        }
    }
}

fn check_status_yaml(doc_root: &Path, result: &mut ValidateOutput) {
    let status_file = doc_root.join("STATUS.yaml");
    if !status_file.exists() {
        return;
    }

    let map = match yaml::read(&status_file) {
        Ok(m) => m,
        Err(_) => return,
    };

    // 必需字段
    for field in &["name", "phase", "mode", "updated", "started"] {
        if !map.contains_key(*field) {
            result.warnings.push(format!("status_missing_field:{}", field));
        }
    }

    // 校验 phase 值
    if let Some(phase) = map.get("phase") {
        let valid = ["PRD", "SPEC", "TASK", "DEV", "TEST", "DONE"];
        if !valid.contains(&phase.as_str()) {
            result.warnings.push(format!("status_invalid_phase:{}", phase));
        }
    }

    // 校验 mode 值
    if let Some(mode) = map.get("mode") {
        let valid_pattern = mode == "full"
            || mode == "quick"
            || mode == "fast"
            || mode == "mvp"
            || mode.starts_with("audit/");
        if !valid_pattern {
            result.warnings.push(format!("status_invalid_mode:{}", mode));
        }
    }
}

fn check_task_files(doc_root: &Path, result: &mut ValidateOutput) {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return;
    }

    let entries = match fs::read_dir(&task_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }

        // 命名规范检查
        let valid_name = name.starts_with("task_") || name.starts_with("done_task_");
        if valid_name {
            let date_part = if name.starts_with("done_task_") {
                &name[10..]
            } else {
                &name[5..]
            };
            // 格式：YYYY-MM-DD_N.md
            let re_ok = date_part.len() >= 13
                && date_part.chars().nth(4) == Some('-')
                && date_part.chars().nth(7) == Some('-')
                && date_part.chars().nth(10) == Some('_');
            if !re_ok {
                result.needs_confirm.push(format!("task_bad_name:{}", name));
            }
        } else {
            result.needs_confirm.push(format!("task_bad_name:{}", name));
            continue;
        }

        // 内容检查
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let task_count = content.lines().filter(|l| l.starts_with("- [")).count();
        let done_when_count = content.lines().filter(|l| l.contains("done_when:")).count();
        let priority_count = content.lines().filter(|l| l.contains("priority:")).count();

        if task_count > 0 && done_when_count < task_count {
            result.warnings.push(format!(
                "task_missing_done_when:{}:{}",
                name,
                task_count - done_when_count
            ));
        }
        if task_count > 0 && priority_count < task_count {
            result.warnings.push(format!(
                "task_missing_priority:{}:{}",
                name,
                task_count - priority_count
            ));
        }
    }
}

fn check_issue_files(doc_root: &Path, result: &mut ValidateOutput) {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return;
    }

    let entries = match fs::read_dir(&issue_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let total: usize = content.lines().filter(|l| l.starts_with("- [")).count();
        let done: usize = content.lines().filter(|l| l.starts_with("- [x]")).count();

        // checkbox 与 prefix 一致性
        if total > 0 && total == done && !name.starts_with("closed_") {
            result.needs_confirm.push(format!("issue_should_be_closed:{}", name));
        }
        if name.starts_with("closed_") && total > 0 && done < total {
            result.needs_confirm.push(format!("issue_closed_but_open_items:{}", name));
        }
    }
}

fn check_changelog(doc_root: &Path, result: &mut ValidateOutput) {
    let changelog = doc_root.join("CHANGELOG.md");
    if changelog.exists() {
        if fs::metadata(&changelog).map(|m| m.len() == 0).unwrap_or(true) {
            result.warnings.push("changelog_empty".to_string());
        }
    } else {
        fs::write(&changelog, "# Changelog\n").ok();
        result.auto_fixed.push("created_changelog".to_string());
    }
}

/// doc_root 为分支子目录时，检查 dev-doc/ 根级是否残留 issue/task 文件
fn check_stale_root_files(doc_root: &Path, result: &mut ValidateOutput) {
    let base_path = Path::new("dev-doc");
    if doc_root == base_path {
        return;
    }

    for subdir in &["issue", "task"] {
        let root_dir = base_path.join(subdir);
        if !root_dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&root_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            result.needs_confirm.push(format!(
                "stale_root_{}:{} → {}/{}/",
                subdir,
                name,
                doc_root.display(),
                subdir
            ));
        }
    }
}

fn check_gitignore(result: &mut ValidateOutput) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp/"
    } else {
        "tmp/"
    };

    if Path::new(".gitignore").exists() {
        let content = fs::read_to_string(".gitignore").unwrap_or_default();
        if !content.lines().any(|l| l.trim() == project_temp) {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(project_temp);
            new_content.push('\n');
            fs::write(".gitignore", new_content).ok();
            result.auto_fixed.push("gitignore_added_project_temp".to_string());
        }
    } else {
        fs::write(".gitignore", format!("{}\n", project_temp)).ok();
        result.auto_fixed.push("gitignore_created".to_string());
    }
}

fn print_human(result: &ValidateOutput) {
    println!("[dev-flow] dev-doc 校验报告");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("文档根：{}", result.doc_root);
    println!();

    if !result.auto_fixed.is_empty() {
        println!("自动修复（{}项）：", result.auto_fixed.len());
        for item in &result.auto_fixed {
            println!("  - {}", item);
        }
        println!();
    }

    if !result.needs_confirm.is_empty() {
        println!("需要确认（{}项）：", result.needs_confirm.len());
        for item in &result.needs_confirm {
            println!("  - {}", item);
        }
        println!();
    }

    if !result.warnings.is_empty() {
        println!("警告（{}项）：", result.warnings.len());
        for item in &result.warnings {
            println!("  - {}", item);
        }
        println!();
    }

    if result.needs_confirm.is_empty() && result.warnings.is_empty() {
        println!("全部通过，无需操作。");
    }
}
