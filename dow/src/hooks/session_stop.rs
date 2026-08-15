// dow/src/hooks/
// ├── session_stop.rs  -- dow hooks session-stop (unified session end handler)
//    1. Revoke claims held by current agent
//    2. Append CHANGELOG record (topic inference: open issue → open task → new commit)
//
// Related Docs:
// - [Claim Core](../core/claim.rs)
// - [CHANGELOG specification](../../references/binary/.dev-doc/CHANGELOG.md)

use crate::core::{claim, doc_root};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct CodexStopOutput {}

pub fn run(codex_hook: bool, _kiro_hook: bool) -> Result<i32, DowError> {
    if !Path::new(crate::core::DOC_DIR).is_dir() {
        print_codex_stop(codex_hook);
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    // Phase 1: Revoke claims held by current agent
    revoke_agent_claims(&doc_root_path, codex_hook);

    // Phase 2: Save changelog
    save_changelog(&doc_root_path, codex_hook)?;

    print_codex_stop(codex_hook);
    Ok(0)
}

/// Revoke all claims belonging to the current agent.
/// If agent_id is undetectable, revoke all claims as fallback.
fn revoke_agent_claims(doc_root: &Path, codex_hook: bool) {
    let agent_id = claim::detect_agent_id();
    let revoked = claim::revoke_by_agent(doc_root, agent_id.as_deref()).unwrap_or_default();

    if !revoked.is_empty() && !codex_hook {
        println!(
            "[dev-flow] Claims revoked on session stop: {}",
            revoked.join(", ")
        );
    }
}

/// Append a CHANGELOG entry based on inferred topic.
fn save_changelog(doc_root: &Path, codex_hook: bool) -> Result<(), DowError> {
    let changelog = doc_root.join("CHANGELOG.md");
    let date = Local::now().format("%Y-%m-%d").to_string();
    let time = Local::now().format("%H:%M").to_string();

    let topic = match infer_topic(doc_root, &changelog) {
        Some(t) => t,
        None => return Ok(()),
    };

    if !changelog.exists() {
        fs::write(&changelog, "# Changelog\n\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let mut content =
        fs::read_to_string(&changelog).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Deduplicate: skip if topic already exists in CHANGELOG
    if content.contains(&topic) {
        return Ok(());
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

    if !codex_hook {
        println!("[dev-flow] CHANGELOG updated: {} {}", time, topic);
    }
    Ok(())
}

fn print_codex_stop(codex_hook: bool) {
    if codex_hook {
        output::print_json(&CodexStopOutput {});
    }
}

/// Infer topic: priority issue title → task title → new commit message
/// issue → "fix: <title>", task → "<type>: <title>", commit → as-is
fn infer_topic(doc_root: &Path, changelog: &Path) -> Option<String> {
    if let Some(topic) = get_issue_topic(doc_root) {
        return Some(format!("fix: {}", topic));
    }
    if let Some((task_type, title)) = get_task_topic(doc_root) {
        return Some(format!("{}: {}", task_type, title));
    }
    get_new_commit_topic(changelog)
}

/// Get title of highest severity open issue
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

/// Get title of highest priority open task, return (type, title)
fn get_task_topic(doc_root: &Path) -> Option<(String, String)> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return None;
    }

    let mut best: Option<(u8, String, String)> = None;

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

/// Get latest commit message (only if not already in CHANGELOG)
/// Skip Release commits generated by iterate
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

    // Skip Release commits generated by iterate
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

/// Find priority rank of field in entry's subsequent lines (P0=0, P1=1, P2=2, default=3)
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

/// Find field value in entry's subsequent lines (e.g., "type: feat" → "feat")
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
