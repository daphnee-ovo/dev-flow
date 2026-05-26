// dow/src/commands/
// ├── issue.rs  -- dow issue（issue 管理）

use crate::cli::IssueArgs;
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
struct IssueListOutput {
    open: Vec<IssueEntry>,
    total: u32,
}

#[derive(Serialize)]
struct IssueEntry {
    file: String,
    items: Vec<IssueItem>,
}

#[derive(Serialize)]
struct IssueItem {
    title: String,
    severity: String,
}

pub fn run(args: IssueArgs, human: bool) -> Result<i32, DowError> {
    if args.list {
        return list_issues(human);
    }

    Err(DowError::new("用法：dow issue --list", 1))
}

fn list_issues(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve("dev-doc");
    let issue_dir = doc_root_path.join("issue");

    let mut entries = Vec::new();

    if issue_dir.is_dir() {
        if let Ok(files) = fs::read_dir(&issue_dir) {
            let mut file_list: Vec<_> = files
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.starts_with("issue_") && name.ends_with(".md")
                })
                .collect();
            file_list.sort_by_key(|e| e.file_name());

            for entry in file_list {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let items = parse_open_items(&content);
                    if !items.is_empty() {
                        entries.push(IssueEntry { file: name, items });
                    }
                }
            }
        }
    }

    let total = entries.iter().map(|e| e.items.len() as u32).sum();
    let result = IssueListOutput { open: entries, total };

    if human {
        if result.total == 0 {
            println!("[dev-flow] 无未关闭的 issue");
        } else {
            println!("[dev-flow] 未关闭 issue：{} 条", result.total);
            println!("━━━━━━━━━━━━━━━━━━━━━━");
            for entry in &result.open {
                println!("{}:", entry.file);
                for item in &entry.items {
                    let sev = if item.severity.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", item.severity)
                    };
                    println!("  - {}{}", item.title, sev);
                }
            }
        }
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn parse_open_items(content: &str) -> Vec<IssueItem> {
    let mut items = Vec::new();
    let mut current_title = String::new();
    let mut in_open = false;

    for line in content.lines() {
        if line.starts_with("- [ ]") {
            in_open = true;
            current_title = line[5..].trim().to_string();
        } else if line.starts_with("- [x]") {
            in_open = false;
            current_title.clear();
        } else if in_open {
            if line.contains("severity:") {
                let sev = line.split("severity:").nth(1).unwrap_or("").trim().to_string();
                items.push(IssueItem {
                    title: current_title.clone(),
                    severity: sev,
                });
                in_open = false;
                current_title.clear();
            }
        }
    }

    // 没有 severity 行的 open item 也要收集
    if in_open && !current_title.is_empty() {
        items.push(IssueItem {
            title: current_title,
            severity: String::new(),
        });
    }

    items
}
