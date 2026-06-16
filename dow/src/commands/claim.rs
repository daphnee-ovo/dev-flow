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
        let target = args.ids.first().map(|s| s.as_str());
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
        claim::add_claims(&doc_root_path, &args.ids).map_err(|e| {
            DowError::new(format!("添加 claim 失败: {}", e), 1)
        })?;

        if human {
            println!("[dev-flow] 已 claim: {}", args.ids.join(", "));
        } else {
            output::print_json(&serde_json::json!({"claimed": &args.ids}));
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
