// dow/src/core/
// ├── claim.rs  -- claim.lock read/write and validation
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
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

/// Return all non-expired claim IDs
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

/// Check if there are expired (but not revoked) claims
pub fn has_expired_claims(doc_root: &Path) -> bool {
    let lock = match read_claim_lock(doc_root) {
        Some(l) => l,
        None => return false,
    };
    lock.claims.iter().any(|c| !is_valid_claim(c, lock.ttl))
}

/// Get agent_id from active claims (returns the first one found, since typically one agent holds claims)
pub fn get_claim_agent_id(doc_root: &Path) -> Option<String> {
    let lock = read_claim_lock(doc_root)?;
    lock.claims
        .iter()
        .filter(|c| is_valid_claim(c, lock.ttl))
        .find_map(|c| c.agent_id.clone())
}

/// Detect current agent ID.
/// Priority: DOW_AGENT_ID env → TTY (Unix) → caller process ID
pub fn detect_agent_id() -> Option<String> {
    // Explicit override via environment variable
    if let Ok(id) = std::env::var("DOW_AGENT_ID") {
        if !id.is_empty() {
            return Some(id);
        }
    }

    // TTY (Unix only — interactive terminals)
    #[cfg(unix)]
    if let Some(tty) = detect_tty() {
        return Some(tty);
    }

    // Fallback: caller process ID (the agent runtime that invoked dow)
    Some(format!("pid:{}", get_caller_pid()))
}

fn get_caller_pid() -> u32 {
    #[cfg(unix)]
    {
        std::os::unix::process::parent_id()
    }
    #[cfg(not(unix))]
    {
        std::process::id()
    }
}

#[cfg(unix)]
fn detect_tty() -> Option<String> {
    use std::process::Command;
    let output = Command::new("tty").output().ok()?;
    if output.status.success() {
        let tty = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if tty == "not a tty" || tty.is_empty() {
            return None;
        }
        Some(tty)
    } else {
        None
    }
}

/// Add claim (merge into existing list, update timestamp if already exists)
pub fn add_claims(doc_root: &Path, ids: &[String]) -> std::io::Result<()> {
    add_claims_with_options(doc_root, ids, detect_agent_id(), None)
}

pub fn add_claims_with_agent(
    doc_root: &Path,
    ids: &[String],
    agent_id: Option<String>,
) -> std::io::Result<()> {
    add_claims_with_options(doc_root, ids, agent_id, None)
}

pub fn add_claims_with_options(
    doc_root: &Path,
    ids: &[String],
    agent_id: Option<String>,
    ttl_override: Option<u64>,
) -> std::io::Result<()> {
    let mut lock = read_claim_lock(doc_root).unwrap_or_else(ClaimLock::empty);
    let ts = now_ts();

    if let Some(ttl) = ttl_override {
        lock.ttl = ttl;
    }

    for id in ids {
        if let Some(existing) = lock.claims.iter_mut().find(|c| &c.id == id) {
            existing.ts = ts;
            existing.agent_id = agent_id.clone();
        } else {
            lock.claims.push(Claim {
                id: id.clone(),
                ts,
                agent_id: agent_id.clone(),
            });
        }
    }

    write_claim_lock(doc_root, &lock)
}

/// Release specified claim; if id is None, release all claims
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
