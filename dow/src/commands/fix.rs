// dow/src/commands/
// ├── fix.rs  -- dow fix（自动修复 dev-doc 文件格式问题）
//
// Related Docs:
// - [ISSUE 规范](../../../references/dev-doc/ISSUE.md)
// - [TASK 规范](../../../references/dev-doc/TASK-FILE.md)

use crate::core::{doc_root, doc_validator};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct FixOutput {
    fixed: Vec<String>,
    unfixable: Vec<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve("dev-doc");
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();

    // 修复 issue 文件
    let issue_dir = doc_root_path.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("issue_") || name.starts_with("closed_issue_"))
                {
                    let results = fix_issue_file(&entry.path());
                    fixed.extend(results.0);
                    unfixable.extend(results.1);
                }
            }
        }
    }

    // 修复 task 文件
    let task_dir = doc_root_path.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("task_") || name.starts_with("done_task_"))
                {
                    let results = fix_task_file(&entry.path());
                    fixed.extend(results.0);
                    unfixable.extend(results.1);
                }
            }
        }
    }

    let result = FixOutput { fixed, unfixable };

    if human {
        print_human(&result);
    } else {
        output::print_json(&result);
    }

    if result.unfixable.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// 修复单个 issue 文件，返回 (已修复, 无法修复) 列表
fn fix_issue_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_issue_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                // 从文件名提取 source，补 frontmatter
                let source = extract_source_from_filename(&filename).unwrap_or("other");
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\nsource: {}\nnums: {}\n---\n\n", source, item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}：补充 frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("source") {
                    let source = extract_source_from_filename(&filename).unwrap_or("other");
                    new_content = insert_fm_field(&new_content, "source", source);
                    needs_write = true;
                    fixed.push(format!("{}：补充 source 字段", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}：补充 nums 字段", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}：{}", filename, error.message));
            }
        }
    }

    if needs_write {
        fs::write(path, &new_content).ok();
    }

    (fixed, unfixable)
}

/// 修复单个 task 文件，返回 (已修复, 无法修复) 列表
fn fix_task_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_task_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\ntitle: TASK - \nnums: {}\n---\n\n", item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}：补充 frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("title") {
                    new_content = insert_fm_field(&new_content, "title", "TASK - ");
                    needs_write = true;
                    fixed.push(format!("{}：补充 title 字段", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}：补充 nums 字段", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}：{}", filename, error.message));
            }
        }
    }

    if needs_write {
        fs::write(path, &new_content).ok();
    }

    (fixed, unfixable)
}

/// 从 issue 文件名提取 source
fn extract_source_from_filename(filename: &str) -> Option<&str> {
    let stem = filename.strip_suffix(".md")?;
    let rest = if stem.starts_with("closed_issue_") {
        &stem["closed_issue_".len()..]
    } else if stem.starts_with("issue_") {
        &stem["issue_".len()..]
    } else {
        return None;
    };
    // source_YYYY-MM-DD_seq
    rest.split('_').next()
}

/// 在已有 frontmatter 中插入字段（如果 frontmatter 存在但缺少字段）
fn insert_fm_field(content: &str, key: &str, value: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    if let Some(end_idx) = content[3..].find("---") {
        let fm_end = 3 + end_idx;
        let mut fm = content[3..fm_end].to_string();
        fm.push_str(&format!("{}: {}\n", key, value));
        format!("---{}---{}", fm, &content[fm_end + 3..])
    } else {
        content.to_string()
    }
}

fn print_human(result: &FixOutput) {
    println!("[dev-flow] 文档格式修复");
    println!("━━━━━━━━━━━━━━━━━━━━━━");

    if result.fixed.is_empty() && result.unfixable.is_empty() {
        println!("所有文件格式正确，无需修复。");
        return;
    }

    if !result.fixed.is_empty() {
        println!("已修复（{}项）：", result.fixed.len());
        for item in &result.fixed {
            println!("  ✓ {}", item);
        }
        println!();
    }

    if !result.unfixable.is_empty() {
        println!("需手动修复（{}项）：", result.unfixable.len());
        for item in &result.unfixable {
            println!("  ✗ {}", item);
        }
        println!();
        println!("提示：以上问题无法自动修复，请手动编辑对应文件。");
    }
}
