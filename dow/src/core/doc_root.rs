// dow/src/core/
// ├── doc_root.rs  -- doc_root resolution logic (corresponds to devflow_resolve_doc_root)
//
// Related Docs:
// - [CLAUDE.md - Directory Structure Convention](../../../CLAUDE.md#directory-structure-convention)

use crate::core::version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve actual doc_root path
/// Enforce .dev-doc/<branch>/ format (including main/master)
/// Automatically create directory and STATUS.yaml for new branches
/// If base is relative, anchors it to project_root() (git toplevel)
pub fn resolve(base: &str) -> PathBuf {
    let raw = Path::new(base);
    let base_path = if raw.is_relative() {
        project_root().join(raw)
    } else {
        raw.to_path_buf()
    };

    if let Some(branch) = current_branch() {
        let branch_path = base_path.join(&branch);
        if branch_path.join("STATUS.yaml").exists() {
            return branch_path;
        }
        // Automatically create branch directory
        if base_path.is_dir() {
            if let Ok(()) = fs::create_dir_all(&branch_path) {
                let status_content = format!(
                    "name: {}\nphase: PRD\nmode: fast\nupdated: {}\nstarted: {}\n",
                    read_project_name(&base_path).unwrap_or_else(|| "project".to_string()),
                    now_str(),
                    now_str(),
                );
                let _ = fs::write(branch_path.join("STATUS.yaml"), &status_content);
                // Initialize version in VERSION for new branch (inherit from main)
                let _ = version::init_branch(&branch);
                return branch_path;
            }
        }
    }

    // Fallback: search subdirectories (when no branch info available)
    if base_path.is_dir() {
        if let Ok(entries) = fs::read_dir(&base_path) {
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| e.path().join("STATUS.yaml").exists())
                .map(|e| e.path())
                .collect();
            found.sort();
            if let Some(first) = found.first() {
                return first.clone();
            }
        }
    }

    base_path.to_path_buf()
}

/// Get project root directory (git repository root)
pub fn project_root() -> PathBuf {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok();
    output
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Get current git branch name
pub fn current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

/// Read project name from STATUS.yaml of existing branch directory
fn read_project_name(base_path: &Path) -> Option<String> {
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let status = entry.path().join("STATUS.yaml");
            if status.exists() {
                if let Ok(content) = fs::read_to_string(&status) {
                    for line in content.lines() {
                        if let Some(name) = line.strip_prefix("name:") {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn now_str() -> String {
    let output = Command::new("date").args(["+%Y-%m-%d %H:%M"]).output().ok();
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
