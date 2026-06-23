// dow/src/commands/
// ├── issue.rs  -- dow issue (issue management)
//
// Related Docs:
// - [ISSUE Specification](../../../references/.dev-doc/ISSUE.md)
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::cli::IssueArgs;
use crate::core::{doc_root, doc_validator};
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

    Err(DowError::new("Usage: dow issue --list", 1))
}

fn list_issues(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    // Validity validation
    let validation_errors = doc_validator::validate_all_issues(&doc_root_path);
    if !validation_errors.is_empty() {
        let msg = doc_validator::format_errors_human(&validation_errors);
        return Err(DowError::new(msg, 1));
    }

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
            println!("[dev-flow] No open issues");
        } else {
            println!("[dev-flow] Open issues: {}", result.total);
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

    // Collect open items without severity line as well
    if in_open && !current_title.is_empty() {
        items.push(IssueItem {
            title: current_title,
            severity: String::new(),
        });
    }

    items
}
