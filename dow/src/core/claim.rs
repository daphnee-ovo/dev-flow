// dow/src/core/
// ├── claim.rs  -- claim.lock read/write and validation
//
// Related Docs:
// - [Guard Hook](../hooks/guard.rs)

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const CLAIM_FILE: &str = "claim.lock";
const DEFAULT_TTL: u64 = 600;

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

/// Return active claim IDs that belong to the current agent.
/// Uses identity matching: direct match, ancestor-chain PID verification,
/// or dead-process takeover (stale claims are considered "ours" to allow recovery).
pub fn get_claims_for_current_agent(doc_root: &Path) -> Vec<String> {
    let lock = match read_claim_lock(doc_root) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let current_agent = match detect_agent_id() {
        Some(id) => id,
        None => return Vec::new(),
    };

    lock.claims
        .iter()
        .filter(|c| is_valid_claim(c, lock.ttl))
        .filter(|c| is_claim_owned_by(&current_agent, c))
        .map(|c| c.id.clone())
        .collect()
}

/// Determine if a claim belongs to the given agent identity.
/// Matching rules:
/// 1. Direct match (same agent_id string)
/// 2. Ancestor-chain: claimed PID is an ancestor of current process
/// 3. Dead process: claimed PID is no longer alive (stale → allow takeover)
fn is_claim_owned_by(current_agent: &str, claim: &Claim) -> bool {
    let claim_agent = match &claim.agent_id {
        Some(id) => id,
        None => return false,
    };

    // Direct match
    if claim_agent == current_agent {
        return true;
    }

    // Ancestor-chain verification for PID-based identities
    if let Some(claimed_pid) = super::process::parse_pid_agent_id(claim_agent) {
        if super::process::is_ancestor_of_current(claimed_pid) {
            return true;
        }
        // Dead process → stale claim, allow takeover
        if !super::process::is_process_alive(claimed_pid) {
            return true;
        }
    }

    false
}

/// Detect current agent ID.
/// Priority: DOW_AGENT_ID env → TTY (Unix) → stable ancestor PID → caller PID
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

    // Walk up process tree to find a stable (non-shell) ancestor
    if let Some(ancestor_pid) = super::process::find_stable_ancestor() {
        return Some(format!("pid:{}", ancestor_pid));
    }

    // Final fallback: caller process ID
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

/// Revoke all claims held by a specific agent.
/// If agent_id is None, revoke all claims (fallback for undetectable agent).
pub fn revoke_by_agent(doc_root: &Path, agent_id: Option<&str>) -> std::io::Result<Vec<String>> {
    let agent_id = match agent_id {
        Some(id) => id,
        None => {
            // Cannot detect agent — revoke all as fallback
            let revoked = get_active_claims(doc_root);
            remove_claim_lock(doc_root);
            return Ok(revoked);
        }
    };

    let mut lock = match read_claim_lock(doc_root) {
        Some(l) => l,
        None => return Ok(Vec::new()),
    };

    let revoked: Vec<String> = lock
        .claims
        .iter()
        .filter(|c| c.agent_id.as_deref() == Some(agent_id))
        .map(|c| c.id.clone())
        .collect();

    if revoked.is_empty() {
        return Ok(Vec::new());
    }

    lock.claims
        .retain(|c| c.agent_id.as_deref() != Some(agent_id));

    if lock.claims.is_empty() {
        remove_claim_lock(doc_root);
    } else {
        write_claim_lock(doc_root, &lock)?;
    }

    Ok(revoked)
}
