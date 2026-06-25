// dow/src/commands/
// ├── claim.rs  -- dow claim (declare current work association)
//
// Related Docs:
// - [Guard Hook](../hooks/guard.rs)
// - [Claim Core](../core/claim.rs)

use crate::core::{claim, doc_root};
use crate::error::DowError;
use crate::output;
use crate::cli::ClaimArgs;
use serde::Serialize;
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
