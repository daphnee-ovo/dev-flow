// dow/src/hooks/
// ├── context.rs  -- dow hooks context (inject context, replaces inject-context.sh)
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)
// - [CODEX_HOOK_ISSUE.md](../../../CODEX_HOOK_ISSUE.md)

use crate::core::{doc_root, version, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct ContextOutput {
    branch: String,
    version: String,
    version_tag: String,
    mode: String,
    phase: String,
    exec_mode: String,
    doc_root: String,
    tasks: TaskStats,
    issues: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    goals_minor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goals_major: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_items: Option<CurrentItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_changelog: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard_notice: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexUserPromptSubmitOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    decision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    hook_specific_output: CodexUserPromptSubmitHookSpecificOutput,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexUserPromptSubmitHookSpecificOutput {
    hook_event_name: &'static str,
    additional_context: String,
}

#[derive(Serialize)]
struct TaskStats {
    total: u32,
    done: u32,
    by_priority: std::collections::BTreeMap<String, PriorityStats>,
}

#[derive(Serialize)]
struct PriorityStats {
    total: u32,
    done: u32,
}

#[derive(Serialize)]
struct CurrentItems {
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<String>,
    #[serde(rename = "type")]
    item_type: String,
    items: Vec<String>,
}

pub fn run(human: bool, codex_hook: bool, kiro_hook: bool) -> Result<i32, DowError> {
    if !Path::new(crate::core::DOC_DIR).is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Ok(0);
    }

    let map = yaml::read(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;
    let phase = map.get("phase").cloned().unwrap_or_default();
    let mode = map.get("mode").cloned().unwrap_or_default();
    let exec_mode = map
        .get("exec_mode")
        .cloned()
        .unwrap_or_else(|| "step".to_string());

    // Count tasks
    let (task_stats, _total, _done) = count_tasks(&doc_root_path);

    // Count open issues
    let open_issues = count_open_issues(&doc_root_path);

    // BLOCKED check: block DEV phase when no pending work
    let mut guard_notice = None;
    if phase == "DEV" && !mode.starts_with("audit/") {
        let undone_items = count_undone_in_active_tasks(&doc_root_path);

        if undone_items == 0 && open_issues == 0 {
            let reason =
                "[dev-flow] DEV phase has no pending tasks or open issues, development not allowed. Please choose:\n\
                → `dow task create` to create new task\n\
                → `dow issue create` to create issue\n\
                → /test to enter test phase\n\
                → `dow status set --phase <PHASE>` to switch phase (PRD/SPEC/TASK/TEST/ITERATE)\n\
                IMPORTANT: Do NOT create tasks/issues and start coding without explicit user approval. Ask the user what they want to do first.";
            if human {
                println!("{}", reason);
            } else if codex_hook {
                guard_notice = Some(reason.to_string());
            } else if kiro_hook {
                let block_json = serde_json::json!({
                    "decision": "block",
                    "reason": reason
                });
                println!("{}", serde_json::to_string_pretty(&block_json).unwrap());
            } else {
                let block_json = serde_json::json!({
                    "decision": "block",
                    "reason": reason
                });
                println!("{}", block_json);
            }
            if !codex_hook {
                return Ok(0);
            }
        }
    }

    // Get current items (issues take priority over tasks)
    let current_items = get_current_items(&doc_root_path, open_issues);

    // Last CHANGELOG entry
    let last_changelog = get_last_changelog(&doc_root_path);

    // Version info
    let (version, version_tag) = read_version_info();

    let branch = doc_root::current_branch().unwrap_or_else(|| "unknown".to_string());

    let goals_minor = map.get("goals_minor").cloned().filter(|s| !s.is_empty());
    let goals_major = map.get("goals_major").cloned().filter(|s| !s.is_empty());

    let blocked = guard_notice.is_some();
    let reason = guard_notice.clone();

    let output_data = ContextOutput {
        branch,
        version,
        version_tag,
        mode,
        phase,
        exec_mode,
        doc_root: doc_root_path.strip_prefix(doc_root::project_root())
            .unwrap_or(&doc_root_path)
            .to_string_lossy().to_string(),
        tasks: task_stats,
        issues: open_issues,
        goals_minor,
        goals_major,
        current_items,
        last_changelog,
        guard_notice,
        blocked,
        reason,
    };

    if codex_hook {
        print_codex_context(&output_data)?;
    } else if kiro_hook {
        output::print_json(&output_data);
    } else if human {
        print_human(&output_data);
    } else {
        output::print_json(&output_data);
    }
    Ok(0)
}

fn count_tasks(doc_root: &Path) -> (TaskStats, u32, u32) {
    let task_dir = doc_root.join("task");
    let mut total = 0u32;
    let mut done = 0u32;
    let mut by_priority: std::collections::BTreeMap<String, PriorityStats> =
        std::collections::BTreeMap::new();

    if !task_dir.is_dir() {
        return (
            TaskStats {
                total: 0,
                done: 0,
                by_priority,
            },
            0,
            0,
        );
    }

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let mut current_done = false;
                for line in content.lines() {
                    if line.starts_with("- [x]") {
                        total += 1;
                        done += 1;
                        current_done = true;
                    } else if line.starts_with("- [ ]") {
                        total += 1;
                        current_done = false;
                    } else if line.contains("priority:") {
                        if let Some(p) = extract_priority(line) {
                            let entry = by_priority
                                .entry(p)
                                .or_insert(PriorityStats { total: 0, done: 0 });
                            entry.total += 1;
                            if current_done {
                                entry.done += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    (
        TaskStats {
            total,
            done,
            by_priority,
        },
        total,
        done,
    )
}

fn extract_priority(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(pos) = trimmed.find("priority:") {
        let val = trimmed[pos + 9..].trim();
        if val.starts_with("P0") || val.starts_with("P1") || val.starts_with("P2") {
            return Some(val[..2].to_string());
        }
    }
    None
}

fn count_open_issues(doc_root: &Path) -> u32 {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return 0;
    }

    let mut open = 0u32;
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("issue_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                open += content.lines().filter(|l| l.starts_with("- [ ]")).count() as u32;
            }
        }
    }
    open
}

/// Count undone checklist items in active task files
fn count_undone_in_active_tasks(doc_root: &Path) -> u32 {
    let task_dir = doc_root.join("task");
    crate::core::task_store::count_undone_items(&task_dir)
}

fn get_current_items(doc_root: &Path, open_issues: u32) -> Option<CurrentItems> {
    if open_issues > 0 {
        // show highest priority issues
        return get_current_issues(doc_root);
    }
    // show highest priority incomplete tasks
    get_current_tasks(doc_root)
}

fn get_current_issues(doc_root: &Path) -> Option<CurrentItems> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return None;
    }

    for severity in &["P0", "P1", "P2"] {
        let mut items = Vec::new();
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("issue_") || !name.ends_with(".md") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let mut in_open = false;
                    let mut title = String::new();
                    for line in content.lines() {
                        if line.starts_with("- [ ]") {
                            in_open = true;
                            title = line.trim_start_matches("- [ ] ").to_string();
                        } else if line.starts_with("- [x]") {
                            in_open = false;
                        } else if in_open && line.contains("severity:") && line.contains(severity) {
                            items.push(title.clone());
                            in_open = false;
                        }
                    }
                }
            }
        }
        if !items.is_empty() {
            return Some(CurrentItems {
                priority: None,
                severity: Some(severity.to_string()),
                item_type: "issue".to_string(),
                items,
            });
        }
    }
    None
}

fn get_current_tasks(doc_root: &Path) -> Option<CurrentItems> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return None;
    }

    for priority in &["P0", "P1", "P2"] {
        let mut items = Vec::new();
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("task_") || !name.ends_with(".md") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let mut in_undone = false;
                    let mut title = String::new();
                    let mut matched = false;
                    let mut task_complexity: Option<String> = None;
                    let mut task_refs: Option<String> = None;
                    for line in content.lines() {
                        if line.starts_with("- [ ]") {
                            if matched {
                                items.push(format_task_item(&title, &task_complexity, &task_refs));
                            }
                            in_undone = true;
                            matched = false;
                            task_complexity = None;
                            task_refs = None;
                            title = line.trim_start_matches("- [ ] ").to_string();
                        } else if line.starts_with("- [x]") {
                            if matched {
                                items.push(format_task_item(&title, &task_complexity, &task_refs));
                            }
                            in_undone = false;
                            matched = false;
                            task_complexity = None;
                            task_refs = None;
                        } else if in_undone {
                            if line.contains("priority:") && line.contains(priority) {
                                matched = true;
                            } else if line.contains("complexity:") {
                                task_complexity = extract_complexity(line);
                            } else if line.contains("refs:") {
                                task_refs = extract_refs(line);
                            }
                        }
                    }
                    if matched {
                        items.push(format_task_item(&title, &task_complexity, &task_refs));
                    }
                }
            }
        }
        if !items.is_empty() {
            return Some(CurrentItems {
                priority: Some(priority.to_string()),
                severity: None,
                item_type: "task".to_string(),
                items,
            });
        }
    }
    None
}

// "TASK-T002: xxx" + complexity "M" + refs "SPEC-A01" → "TASK-T002[M]: xxx {refs:[SPEC-A01]}"
fn format_task_item(title: &str, complexity: &Option<String>, refs: &Option<String>) -> String {
    let base = match complexity {
        Some(c) => {
            if let Some(colon_pos) = title.find(':') {
                format!("{}[{}]{}", &title[..colon_pos], c, &title[colon_pos..])
            } else {
                format!("{}[{}]", title, c)
            }
        }
        None => title.to_string(),
    };
    match refs {
        Some(r) => format!("{} {{refs:[{}]}}", base, r),
        None => base,
    }
}

fn extract_refs(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(pos) = trimmed.find("refs:") {
        let val = trimmed[pos + 5..].trim();
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }
    None
}

fn extract_complexity(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(pos) = trimmed.find("complexity:") {
        let val = trimmed[pos + 11..].trim();
        match val {
            "S" | "M" | "L" => Some(val.to_string()),
            _ => None,
        }
    } else {
        None
    }
}

fn get_last_changelog(doc_root: &Path) -> Option<String> {
    let changelog = doc_root.join("CHANGELOG.md");
    if !changelog.exists() {
        return None;
    }
    fs::read_to_string(&changelog).ok().and_then(|c| {
        c.lines()
            .find(|l| l.starts_with("- "))
            .map(|l| l.trim_start_matches("- ").to_string())
    })
}

fn read_version_info() -> (String, String) {
    let ver = version::read_current().unwrap_or_else(|_| "0.0.0".to_string());

    let tag_status = Command::new("git")
        .args(["tag", "-l", &format!("v{}", ver)])
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if out.is_empty() {
                "no-tag".to_string()
            } else {
                "tagged".to_string()
            }
        })
        .unwrap_or_else(|_| "no-tag".to_string());

    (ver, tag_status)
}

fn print_codex_context(data: &ContextOutput) -> Result<(), DowError> {
    let (decision, reason) = if data.guard_notice.is_some() {
        (Some("block"), data.guard_notice.clone())
    } else {
        (None, None)
    };

    let context_json = serde_json::to_string(data)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let output = CodexUserPromptSubmitOutput {
        decision,
        reason,
        hook_specific_output: CodexUserPromptSubmitHookSpecificOutput {
            hook_event_name: "UserPromptSubmit",
            additional_context: context_json,
        },
    };
    output::print_json(&output);
    Ok(())
}

fn print_human(data: &ContextOutput) {
    println!("{}", format_human_context(data));
}

fn format_human_context(data: &ContextOutput) -> String {
    let mut lines = vec![
        format!(
            "[dev-flow] branch:{} | phase:{} | mode:{} | exec:{} | v{} ({})",
            data.branch, data.phase, data.mode, data.exec_mode, data.version, data.version_tag
        ),
        format!(
            "doc_root:{} | tasks:{}/{} | issues:{}",
            data.doc_root, data.tasks.done, data.tasks.total, data.issues
        ),
    ];

    if !data.tasks.by_priority.is_empty() {
        let parts: Vec<String> = data
            .tasks
            .by_priority
            .iter()
            .map(|(k, v)| format!("{}:{}/{}", k, v.done, v.total))
            .collect();
        lines.push(format!("priority: {}", parts.join(" | ")));
    }

    if data.goals_minor.is_some() || data.goals_major.is_some() {
        let mut parts = Vec::new();
        if let Some(ref g) = data.goals_minor {
            parts.push(format!("minor: {}", g));
        }
        if let Some(ref g) = data.goals_major {
            parts.push(format!("major: {}", g));
        }
        lines.push(format!("goals: {}", parts.join(" | ")));
    }

    if let Some(ref items) = data.current_items {
        let label = if items.item_type == "issue" {
            format!(
                "current issues [{}]",
                items.severity.as_deref().unwrap_or("?")
            )
        } else {
            format!(
                "current tasks [{}]",
                items.priority.as_deref().unwrap_or("?")
            )
        };
        lines.push(format!("{}:", label));
        for item in &items.items {
            lines.push(format!("  - {}", item));
        }
    }

    if let Some(ref last) = data.last_changelog {
        lines.push(format!("last: {}", last));
    }

    if let Some(ref notice) = data.guard_notice {
        lines.push(notice.clone());
    }

    lines.join("\n")
}
