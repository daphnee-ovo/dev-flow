// dow/src/hooks/
// ├── context.rs  -- dow hooks context（注入上下文，替代 inject-context.sh）

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct ContextOutput {
    version: String,
    version_tag: String,
    mode: String,
    phase: String,
    exec_mode: String,
    doc_root: String,
    tasks: TaskStats,
    issues: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_items: Option<CurrentItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_changelog: Option<String>,
}

#[derive(Serialize)]
struct BlockedOutput {
    blocked: bool,
    reasons: Vec<String>,
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

pub fn run(_human: bool) -> Result<i32, DowError> {
    if !Path::new("dev-doc").is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve("dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Ok(0);
    }

    let map = yaml::read(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;
    let phase = map.get("phase").cloned().unwrap_or_default();
    let mode = map.get("mode").cloned().unwrap_or_default();
    let exec_mode = map.get("exec_mode").cloned().unwrap_or_else(|| "step".to_string());

    // 统计 tasks
    let (task_stats, total, done) = count_tasks(&doc_root_path);

    // 统计 open issues
    let open_issues = count_open_issues(&doc_root_path);

    // BLOCKED 检查
    if phase == "DEV" && !mode.starts_with("audit/") {
        let active_tasks = count_active_task_files(&doc_root_path);
        let done_tasks = count_done_task_files(&doc_root_path);

        if active_tasks == 0 && open_issues == 0 && done_tasks == 0 {
            let blocked = BlockedOutput {
                blocked: true,
                reasons: vec![
                    "DEV 阶段无活跃 task 且无 open issue".to_string(),
                ],
            };
            output::print_json(&blocked);
            return Ok(1);
        }
    }

    // 获取当前 items（issue 优先于 task）
    let current_items = get_current_items(&doc_root_path, open_issues);

    // 最近 CHANGELOG 条目
    let last_changelog = get_last_changelog(&doc_root_path);

    // 版本信息
    let (version, version_tag) = read_version_info();

    let output_data = ContextOutput {
        version,
        version_tag,
        mode,
        phase,
        exec_mode,
        doc_root: doc_root_path.to_string_lossy().to_string(),
        tasks: task_stats,
        issues: open_issues,
        current_items,
        last_changelog,
    };

    output::print_json(&output_data);
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
            TaskStats { total: 0, done: 0, by_priority },
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
                            let entry = by_priority.entry(p).or_insert(PriorityStats { total: 0, done: 0 });
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

    (TaskStats { total, done, by_priority }, total, done)
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

fn count_active_task_files(doc_root: &Path) -> u32 {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return 0;
    }
    fs::read_dir(&task_dir)
        .map(|e| {
            e.flatten()
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("task_") && n.ends_with(".md")
                })
                .count() as u32
        })
        .unwrap_or(0)
}

fn count_done_task_files(doc_root: &Path) -> u32 {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return 0;
    }
    fs::read_dir(&task_dir)
        .map(|e| {
            e.flatten()
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("done_task_") && n.ends_with(".md")
                })
                .count() as u32
        })
        .unwrap_or(0)
}

fn get_current_items(doc_root: &Path, open_issues: u32) -> Option<CurrentItems> {
    if open_issues > 0 {
        // 显示最高优先级 issue
        return get_current_issues(doc_root);
    }
    // 显示最高优先级未完成 task
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
                    for line in content.lines() {
                        if line.starts_with("- [ ]") {
                            in_undone = true;
                            title = line.trim_start_matches("- [ ] ").to_string();
                        } else if line.starts_with("- [x]") {
                            in_undone = false;
                        } else if in_undone && line.contains("priority:") && line.contains(priority)
                        {
                            items.push(title.clone());
                            in_undone = false;
                        }
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

fn get_last_changelog(doc_root: &Path) -> Option<String> {
    let changelog = doc_root.join("CHANGELOG.md");
    if !changelog.exists() {
        return None;
    }
    fs::read_to_string(&changelog)
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("- "))
                .map(|l| l.trim_start_matches("- ").to_string())
        })
}

fn read_version_info() -> (String, String) {
    let version = fs::read_to_string("VERSION")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0.0.0".to_string());

    let tag_status = Command::new("git")
        .args(["tag", "-l", &format!("v{}", version)])
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if out.is_empty() { "no-tag".to_string() } else { "synced".to_string() }
        })
        .unwrap_or_else(|_| "no-tag".to_string());

    (version, tag_status)
}
