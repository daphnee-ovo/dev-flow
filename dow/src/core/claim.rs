// dow/src/core/
// ├── claim.rs  -- claim.lock 读写与验证
//
// Related Docs:
// - [Guard Hook](../hooks/guard.rs)

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const CLAIM_FILE: &str = "claim.lock";
const DEFAULT_TTL: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimLock {
    pub claims: Vec<Claim>,
    pub ttl: u64,
}

impl ClaimLock {
    pub fn empty() -> Self {
        Self {
            claims: Vec::new(),
            ttl: DEFAULT_TTL,
        }
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn read_claim_lock(doc_root: &Path) -> Option<ClaimLock> {
    let path = doc_root.join(CLAIM_FILE);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_claim_lock(doc_root: &Path, lock: &ClaimLock) -> std::io::Result<()> {
    let path = doc_root.join(CLAIM_FILE);
    let json = serde_json::to_string_pretty(lock)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&path, json)
}

pub fn remove_claim_lock(doc_root: &Path) {
    let path = doc_root.join(CLAIM_FILE);
    let _ = std::fs::remove_file(&path);
}

pub fn is_valid_claim(claim: &Claim, ttl: u64) -> bool {
    let now = now_ts();
    now.saturating_sub(claim.ts) < ttl
}

/// 返回所有尚未过期的 claim ID
pub fn get_active_claims(doc_root: &Path) -> Vec<String> {
    let lock = match read_claim_lock(doc_root) {
        Some(l) => l,
        None => return Vec::new(),
    };
    lock.claims
        .iter()
        .filter(|c| is_valid_claim(c, lock.ttl))
        .map(|c| c.id.clone())
        .collect()
}

/// 添加 claim（合并到现有列表，已存在则更新时间戳）
pub fn add_claims(doc_root: &Path, ids: &[String]) -> std::io::Result<()> {
    let mut lock = read_claim_lock(doc_root).unwrap_or_else(ClaimLock::empty);
    let ts = now_ts();

    for id in ids {
        if let Some(existing) = lock.claims.iter_mut().find(|c| &c.id == id) {
            existing.ts = ts;
        } else {
            lock.claims.push(Claim {
                id: id.clone(),
                ts,
            });
        }
    }

    write_claim_lock(doc_root, &lock)
}

/// 释放指定 claim，若 id 为 None 则全部释放
pub fn revoke_claims(doc_root: &Path, id: Option<&str>) -> std::io::Result<()> {
    match id {
        None => {
            remove_claim_lock(doc_root);
            Ok(())
        }
        Some(target) => {
            let mut lock = read_claim_lock(doc_root).unwrap_or_else(ClaimLock::empty);
            lock.claims.retain(|c| c.id != target);
            if lock.claims.is_empty() {
                remove_claim_lock(doc_root);
                Ok(())
            } else {
                write_claim_lock(doc_root, &lock)
            }
        }
    }
}
