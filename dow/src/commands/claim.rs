// dow/src/commands/
// ├── claim.rs  -- dow claim（声明当前工作关联）
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
            DowError::new(format!("释放 claim 失败: {}", e), 1)
        })?;

        if human {
            match target {
                Some(id) => println!("[dev-flow] 已释放 claim: {}", id),
                None => println!("[dev-flow] 已释放所有 claim"),
            }
        } else {
            output::print_json(&serde_json::json!({"revoked": target.unwrap_or("all")}));
        }
        return Ok(0);
    }

    if !args.ids.is_empty() {
        let normalized: Vec<String> = args.ids.iter().map(|id| normalize_claim_id(id)).collect();

        // 校验每个 ID 是否对应未完成的 task/issue
        let invalid = find_invalid_claim_ids(&doc_root_path, &normalized);
        if !invalid.is_empty() {
            return Err(DowError::new(
                format!("无法 claim 已完成或不存在的项：{}", invalid.join(", ")),
                1,
            ));
        }

        claim::add_claims(&doc_root_path, &normalized).map_err(|e| {
            DowError::new(format!("添加 claim 失败: {}", e), 1)
        })?;

        if human {
            println!("[dev-flow] 已 claim: {}", normalized.join(", "));
        } else {
            output::print_json(&serde_json::json!({"claimed": &normalized}));
        }
        return Ok(0);
    }

    // 无参数：显示当前状态
    let lock = claim::read_claim_lock(&doc_root_path);
    match lock {
        None => {
            if human {
                println!("[dev-flow] 无活跃 claim");
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
                    println!("[dev-flow] 无活跃 claim");
                } else {
                    if !active.is_empty() {
                        println!("[dev-flow] 活跃 claim:");
                        for c in &active {
                            println!("  {} (剩余 {}s)", c.id, c.remaining_secs);
                        }
                    }
                    if !expired.is_empty() {
                        println!("[dev-flow] 已过期:");
                        for c in &expired {
                            println!("  {} (过期 {}s)", c.id, -c.remaining_secs);
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

/// 将全称 ID 归一化为短格式：TASK-T001 → T001, ISSUE-I001 → I001
/// 已经是短格式的直接返回
fn normalize_claim_id(id: &str) -> String {
    if let Some(short) = id.strip_prefix("TASK-") {
        short.to_string()
    } else if let Some(short) = id.strip_prefix("ISSUE-") {
        short.to_string()
    } else {
        id.to_string()
    }
}

/// 检查 claim ID 是否对应未完成的 task 或 open issue
/// 返回无效（已完成/不存在）的 ID 列表
fn find_invalid_claim_ids(doc_root: &std::path::Path, ids: &[String]) -> Vec<String> {
    let mut open_ids: Vec<String> = Vec::new();

    // 收集 task 文件中所有未完成项的 ID
    let task_dir = doc_root.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") {
                    continue;
                }
                // done_task_ 文件 = 已全部完成，跳过
                if name.starts_with("done_") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [ ] TASK-") {
                            // 提取 ID: "- [ ] TASK-T001: ..." → "T001"
                            if let Some(rest) = line.strip_prefix("- [ ] TASK-") {
                                if let Some(id) = rest.split(':').next() {
                                    open_ids.push(id.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 收集 issue 文件中所有未完成项的 ID
    let issue_dir = doc_root.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") {
                    continue;
                }
                if name.starts_with("closed_") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [ ] ISSUE-") {
                            // "- [ ] ISSUE-I001：..." → "I001"
                            if let Some(rest) = line.strip_prefix("- [ ] ISSUE-") {
                                let id = rest.split([':', '：']).next().unwrap_or("").trim();
                                if !id.is_empty() {
                                    open_ids.push(id.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ids.iter()
        .filter(|id| !open_ids.contains(id))
        .cloned()
        .collect()
}
