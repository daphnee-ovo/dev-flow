// dow/src/hooks/
// ├── save_changelog.rs  -- dow hooks save-changelog（会话结束时追加记录）
//    topic 推断：open issue → open task → 新 commit（去重）
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)
// - [CHANGELOG 规范](../../../references/.dev-doc/CHANGELOG.md)

use crate::core::doc_root;
use crate::error::DowError;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run() -> Result<i32, DowError> {
    if !Path::new(crate::core::DOC_DIR).is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let changelog = doc_root_path.join("CHANGELOG.md");
    let date = Local::now().format("%Y-%m-%d").to_string();
    let time = Local::now().format("%H:%M").to_string();

    let topic = match infer_topic(&doc_root_path, &changelog) {
        Some(t) => t,
        None => return Ok(0),
    };

    if !changelog.exists() {
        fs::write(&changelog, "# Changelog\n\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let mut content =
        fs::read_to_string(&changelog).map_err(|e| DowError::new(e.to_string(), 1))?;

    // 去重：topic 已存在于 CHANGELOG 中则跳过
    if content.contains(&topic) {
        return Ok(0);
    }

    let date_header = format!("## {}", date);
    if !content.contains(&date_header) {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("\n{}\n", date_header));
    }

    let entry = format!("- {} {}", time, topic);
    content.push_str(&format!("{}\n", entry));
    fs::write(&changelog, content).map_err(|e| DowError::new(e.to_string(), 1))?;

    println!("[dev-flow] CHANGELOG 已更新：{} {}", time, topic);
    Ok(0)
}

/// 推断 topic：优先 issue 标题 → task 标题 → 新 commit message
/// issue → "fix: <title>", task → "<type>: <title>", commit → 原样
fn infer_topic(doc_root: &Path, changelog: &Path) -> Option<String> {
    if let Some(topic) = get_issue_topic(doc_root) {
        return Some(format!("fix: {}", topic));
    }
    if let Some((task_type, title)) = get_task_topic(doc_root) {
        return Some(format!("{}: {}", task_type, title));
    }
    get_new_commit_topic(changelog)
}

/// 从 open issue 中取最高 severity 的标题
fn get_issue_topic(doc_root: &Path) -> Option<String> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return None;
    }

    let mut best: Option<(u8, String)> = None;

    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("issue_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if line.starts_with("- [ ] ISSUE-") {
                        let title = line.trim_start_matches("- [ ] ").to_string();
                        let rank = find_field_rank(&lines[i..], "severity:");
                        if best.is_none() || rank < best.as_ref().unwrap().0 {
                            let clean = title
                                .trim_end_matches('：')
                                .trim_end_matches(':')
                                .to_string();
                            best = Some((rank, clean));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, title)| title)
}

/// 从 open task 中取最高 priority 的标题，返回 (type, title)
fn get_task_topic(doc_root: &Path) -> Option<(String, String)> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return None;
    }

    let mut best: Option<(u8, String, String)> = None; // (rank, type, title)

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("task_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if line.starts_with("- [ ] TASK-") {
                        let title = line.trim_start_matches("- [ ] ").to_string();
                        let rank = find_field_rank(&lines[i..], "priority:");
                        let task_type = find_field_value(&lines[i..], "type:")
                            .unwrap_or_else(|| "feat".to_string());
                        if best.is_none() || rank < best.as_ref().unwrap().0 {
                            let clean = title
                                .split(':')
                                .nth(1)
                                .or_else(|| title.split('：').nth(1))
                                .map(|s| s.trim().to_string())
                                .unwrap_or(title);
                            best = Some((rank, task_type, clean));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, t, title)| (t, title))
}

/// 获取最新 commit message（仅当 CHANGELOG 中不包含该内容时）
/// 跳过 iterate 产生的 Release commit
fn get_new_commit_topic(changelog: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let (hash, topic) = msg.split_once(' ')?;

    // 跳过 iterate 产生的 Release commit
    if topic.contains("Release v") {
        return None;
    }

    if let Ok(content) = fs::read_to_string(changelog) {
        if content.contains(hash) || content.contains(topic) {
            return None;
        }
    }

    Some(topic.to_string())
}

/// 在条目后续行中查找字段的优先级等级（P0=0, P1=1, P2=2, 默认=3）
fn find_field_rank(lines: &[&str], field: &str) -> u8 {
    for line in lines.iter().skip(1) {
        if line.starts_with("- [") {
            break;
        }
        if line.contains(field) {
            if line.contains("P0") {
                return 0;
            }
            if line.contains("P1") {
                return 1;
            }
            if line.contains("P2") {
                return 2;
            }
            return 3;
        }
    }
    3
}

/// 在条目后续行中查找字段值（如 "type: feat" → "feat"）
fn find_field_value(lines: &[&str], field: &str) -> Option<String> {
    for line in lines.iter().skip(1) {
        if line.starts_with("- [") {
            break;
        }
        if let Some(pos) = line.find(field) {
            let val = line[pos + field.len()..].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}
