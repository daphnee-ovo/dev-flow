// dow/src/commands/
// ├── claim.rs  -- dow claim (declare current work association)
//
// Related Docs:
// - [Guard Hook](../hooks/guard.rs)
// - [Claim Core](../core/claim.rs)

use crate::commands::task as task_cmd;
use crate::core::{claim, doc_root};
use crate::error::DowError;
use crate::output;
use crate::cli::ClaimArgs;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;

#[derive(Serialize)]
struct ClaimStatus {
    active: Vec<ClaimInfo>,
    expired: Vec<ClaimInfo>,
}

#[derive(Serialize)]
struct ClaimInfo {
    id: String,
    ts: u64,
    remaining_secs: i64,
}

pub fn run(args: ClaimArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    if args.revoke {
        let normalized_target = args.ids.first().map(|s| normalize_claim_id(s));
        let target = normalized_target.as_deref();
        claim::revoke_claims(&doc_root_path, target).map_err(|e| {
            DowError::new(format!("Failed to revoke claim: {}", e), 1)
        })?;

        // Silent on success — operator knows what they revoked
        return Ok(0);
    }

    if !args.ids.is_empty() {
        let normalized: Vec<String> = args.ids.iter().map(|id| normalize_claim_id(id)).collect();

        // Validate whether each ID corresponds to an incomplete task/issue
        let (invalid, duplicates) = validate_claim_ids(&doc_root_path, &normalized);
        if !duplicates.is_empty() {
            return Err(DowError::new(
                format!("ID is ambiguous (appears in multiple files): {}. Please manually fix the sequence number to make it unique.", duplicates.join(", ")),
                1,
            ));
        }
        if !invalid.is_empty() {
            return Err(DowError::new(
                format!("Cannot claim completed or non-existent items: {}", invalid.join(", ")),
                1,
            ));
        }

        // Check dependencies before allowing claim
        check_dependencies(&doc_root_path, &normalized)?;

        // Check issue files requirement
        check_issue_files(&doc_root_path, &normalized)?;

        claim::add_claims(&doc_root_path, &normalized).map_err(|e| {
            DowError::new(format!("Failed to add claim: {}", e), 1)
        })?;

        // Silent on success — operator knows what they claimed
        return Ok(0);
    }

    // No arguments: show current status
    let lock = claim::read_claim_lock(&doc_root_path);
    match lock {
        None => {
            if human {
                println!("[dev-flow] No active claims");
            } else {
                output::print_json(&ClaimStatus {
                    active: vec![],
                    expired: vec![],
                });
            }
        }
        Some(lock) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let mut active = Vec::new();
            let mut expired = Vec::new();

            for c in &lock.claims {
                let elapsed = now.saturating_sub(c.ts);
                let remaining = lock.ttl as i64 - elapsed as i64;
                let info = ClaimInfo {
                    id: c.id.clone(),
                    ts: c.ts,
                    remaining_secs: remaining,
                };
                if remaining > 0 {
                    active.push(info);
                } else {
                    expired.push(info);
                }
            }

            if human {
                if active.is_empty() && expired.is_empty() {
                    println!("[dev-flow] No active claims");
                } else {
                    if !active.is_empty() {
                        println!("[dev-flow] Active claims:");
                        for c in &active {
                            println!("  {} ({}s remaining)", c.id, c.remaining_secs);
                        }
                    }
                    if !expired.is_empty() {
                        println!("[dev-flow] Expired:");
                        for c in &expired {
                            println!("  {} (expired {}s ago)", c.id, -c.remaining_secs);
                        }
                    }
                }
            } else {
                output::print_json(&ClaimStatus { active, expired });
            }
        }
    }

    Ok(0)
}

/// Normalize full ID to short format: TASK-T001 → T001, ISSUE-I001 → I001
/// If already in short format, return as-is
fn normalize_claim_id(id: &str) -> String {
    if let Some(short) = id.strip_prefix("TASK-") {
        short.to_string()
    } else if let Some(short) = id.strip_prefix("ISSUE-") {
        short.to_string()
    } else {
        id.to_string()
    }
}

/// Validate claim IDs: returns (invalid_ids, duplicate_ids)
/// invalid = completed or non-existent; duplicate = same ID appears in multiple files
fn validate_claim_ids(doc_root: &std::path::Path, ids: &[String]) -> (Vec<String>, Vec<String>) {
    // id → occurrence count
    let mut id_count: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    // Collect all incomplete task IDs from task files
    let task_dir = doc_root.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") || name.starts_with("done_") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [ ] TASK-") {
                            if let Some(rest) = line.strip_prefix("- [ ] TASK-") {
                                if let Some(id) = rest.split(':').next() {
                                    *id_count.entry(id.trim().to_string()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect all incomplete issue IDs from issue files
    let issue_dir = doc_root.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") || name.starts_with("closed_") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [ ] ISSUE-") {
                            if let Some(rest) = line.strip_prefix("- [ ] ISSUE-") {
                                let id = rest.split([':', '：']).next().unwrap_or("").trim();
                                if !id.is_empty() {
                                    *id_count.entry(id.to_string()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let invalid: Vec<String> = ids.iter()
        .filter(|id| !id_count.contains_key(id.as_str()))
        .cloned()
        .collect();

    let duplicates: Vec<String> = ids.iter()
        .filter(|id| id_count.get(id.as_str()).copied().unwrap_or(0) > 1)
        .cloned()
        .collect();

    (invalid, duplicates)
}

/// Check explicit dependencies (depends_on) and implicit file conflicts (already-claimed tasks)
fn check_dependencies(doc_root: &std::path::Path, ids: &[String]) -> Result<(), DowError> {
    // Only check task IDs (not issues)
    let task_ids: Vec<&String> = ids.iter().filter(|id| id.starts_with("T")).collect();
    if task_ids.is_empty() {
        return Ok(());
    }

    let all_tasks = task_cmd::get_all_task_details(doc_root);

    // Get currently claimed task IDs (excluding the ones being claimed now)
    let currently_claimed: Vec<String> = claim::get_active_claims(doc_root)
        .into_iter()
        .filter(|c| c.starts_with("T") && !ids.contains(c))
        .collect();

    let mut errors: Vec<String> = Vec::new();

    for target_id in &task_ids {
        let full_id = format!("TASK-{}", target_id);
        let target = match all_tasks.iter().find(|t| t.id == full_id) {
            Some(t) => t,
            None => continue,
        };

        // 1. Check explicit depends_on: all must be done
        let incomplete_deps: Vec<&str> = target.depends_on.iter()
            .filter_map(|dep_id| {
                let dep = all_tasks.iter().find(|t| t.id == *dep_id)?;
                if dep.status != "done" { Some(dep_id.as_str()) } else { None }
            })
            .collect();

        if !incomplete_deps.is_empty() {
            errors.push(format!(
                "cannot claim {}: blocked by incomplete dependencies [{}]",
                full_id, incomplete_deps.join(", ")
            ));
        }

        // 2. Check file conflicts with currently claimed tasks
        let target_files: HashSet<&str> = target.files.create.iter()
            .chain(target.files.modify.iter())
            .filter(|f| !f.is_empty())
            .map(|f| f.as_str())
            .collect();

        if target_files.is_empty() {
            continue;
        }

        for claimed_id in &currently_claimed {
            let claimed_full_id = format!("TASK-{}", claimed_id);
            let claimed_task = match all_tasks.iter().find(|t| t.id == claimed_full_id) {
                Some(t) => t,
                None => continue,
            };

            if claimed_task.status == "done" {
                continue;
            }

            let claimed_files: HashSet<&str> = claimed_task.files.create.iter()
                .chain(claimed_task.files.modify.iter())
                .filter(|f| !f.is_empty())
                .map(|f| f.as_str())
                .collect();

            let shared: Vec<&&str> = target_files.intersection(&claimed_files).collect();
            if !shared.is_empty() {
                let shared_list: Vec<&str> = shared.into_iter().copied().collect();
                errors.push(format!(
                    "cannot claim {}: file conflict with currently claimed {} (shared: {})",
                    full_id, claimed_full_id, shared_list.join(", ")
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(DowError::new(errors.join("\n"), 1))
    }
}

/// Check that issue claims have files declared (required for scope tracking)
fn check_issue_files(doc_root: &std::path::Path, ids: &[String]) -> Result<(), DowError> {
    let issue_ids: Vec<&String> = ids.iter().filter(|id| id.starts_with("I")).collect();
    if issue_ids.is_empty() {
        return Ok(());
    }

    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Ok(());
    }

    let mut errors: Vec<String> = Vec::new();

    for iid in &issue_ids {
        let full_id = format!("ISSUE-{}", iid);
        let has_files = check_issue_has_files(&issue_dir, &full_id);
        if !has_files {
            errors.push(format!(
                "cannot claim {}: no files declared. Use `dow issue update {} --files-modify \"path/to/file\"` first.",
                full_id, full_id
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(DowError::new(errors.join("\n"), 1))
    }
}

fn check_issue_has_files(issue_dir: &std::path::Path, target_id: &str) -> bool {
    let entries = match fs::read_dir(issue_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") || name.starts_with("closed_") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let mut in_target = false;
            for line in content.lines() {
                if line.starts_with("- [ ]") && line.contains(target_id) {
                    in_target = true;
                } else if in_target && (line.starts_with("- [ ]") || line.starts_with("- [x]")) {
                    break;
                } else if in_target {
                    let trimmed = line.trim();
                    if trimmed.starts_with("- files_modify:") || trimmed.starts_with("- files_create:") {
                        let after_colon = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                        if after_colon != "[]" && !after_colon.is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
