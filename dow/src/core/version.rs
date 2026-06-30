// dow/src/core/
// ├── version.rs  -- VERSION file multi-branch read/write
//
// Format: Each line (<branch>)<version>, e.g.:
//   (main)3.4.0
//   (feature-x)3.4.0
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::core::doc_root;
use crate::error::DowError;
use std::fs;

struct VersionEntry {
    branch: String,
    version: String,
}

/// Read current branch version number. Falls back to 'main' on detached HEAD.
pub fn read_current() -> Result<String, DowError> {
    let branch = resolve_branch();
    read_branch(&branch)
}

/// Resolve branch: current branch or fallback to 'main'
pub fn resolve_branch() -> String {
    doc_root::current_branch().unwrap_or_else(|| "main".to_string())
}

/// Whether the current state is a detached HEAD (no branch detected)
pub fn is_detached() -> bool {
    doc_root::current_branch().is_none()
}

/// Read specified branch version number
pub fn read_branch(branch: &str) -> Result<String, DowError> {
    let entries = parse_file()?;
    entries
        .iter()
        .find(|e| e.branch == branch)
        .map(|e| e.version.clone())
        .ok_or_else(|| DowError::new(format!("No record for branch {} in VERSION", branch), 1))
}

/// Write current branch version number (update existing line or append new line)
pub fn write_current(version: &str) -> Result<(), DowError> {
    let branch = doc_root::current_branch()
        .ok_or_else(|| DowError::new("Failed to get current branch", 1))?;
    write_branch(&branch, version)
}

/// Write specified branch version number
pub fn write_branch(branch: &str, version: &str) -> Result<(), DowError> {
    validate_semver(version)?;
    let mut entries = parse_file().unwrap_or_default();

    if let Some(entry) = entries.iter_mut().find(|e| e.branch == branch) {
        entry.version = version.to_string();
    } else {
        entries.push(VersionEntry {
            branch: branch.to_string(),
            version: version.to_string(),
        });
    }

    write_file(&entries)
}

/// Delete specified branch version record
pub fn remove_branch(branch: &str) -> Result<(), DowError> {
    let mut entries = parse_file().unwrap_or_default();
    entries.retain(|e| e.branch != branch);
    write_file(&entries)
}

/// Initialize version for new branch (inherit from main)
pub fn init_branch(branch: &str) -> Result<String, DowError> {
    let entries = parse_file().unwrap_or_default();

    // If already exists, return directly
    if let Some(entry) = entries.iter().find(|e| e.branch == branch) {
        return Ok(entry.version.clone());
    }

    // Inherit version from main/master
    let base_version = entries
        .iter()
        .find(|e| e.branch == "main" || e.branch == "master")
        .map(|e| e.version.clone())
        .unwrap_or_else(|| "0.1.0".to_string());

    write_branch(branch, &base_version)?;
    Ok(base_version)
}

/// Bump current branch version number (read + calculate + write)
pub fn bump(bump_type: &str) -> Result<(String, String), DowError> {
    let current = read_current()?;
    let new = bump_version(&current, bump_type)?;
    write_current(&new)?;
    Ok((current, new))
}

/// Pure calculation: given version number + bump type, return new version number (don't write to file)
pub fn bump_version_str(version: &str, bump_type: &str) -> Result<String, DowError> {
    bump_version(version, bump_type)
}

/// Parse version file, compatible with old format (plain version number, treated as main)
fn parse_file() -> Result<Vec<VersionEntry>, DowError> {
    let content = fs::read_to_string("VERSION")
        .map_err(|_| DowError::new("VERSION file does not exist or is not readable", 1))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(entry) = parse_line(trimmed) {
            entries.push(entry);
        } else if is_plain_semver(trimmed) {
            // Old format compatibility: plain version number treated as main
            entries.push(VersionEntry {
                branch: "main".to_string(),
                version: trimmed.to_string(),
            });
        }
    }

    if entries.is_empty() {
        return Err(DowError::new("VERSION file is empty or format cannot be parsed", 1));
    }

    Ok(entries)
}

/// Parse single line: (branch)version
fn parse_line(line: &str) -> Option<VersionEntry> {
    if !line.starts_with('(') {
        return None;
    }
    let close = line.find(')')?;
    let branch = &line[1..close];
    let version = &line[close + 1..];

    if branch.is_empty() || version.is_empty() {
        return None;
    }

    Some(VersionEntry {
        branch: branch.to_string(),
        version: version.to_string(),
    })
}

fn is_plain_semver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())
}

fn write_file(entries: &[VersionEntry]) -> Result<(), DowError> {
    let content: String = entries
        .iter()
        .map(|e| format!("({}){}", e.branch, e.version))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    fs::write("VERSION", content)
        .map_err(|e| DowError::new(e.to_string(), 1))
}

fn validate_semver(version: &str) -> Result<(), DowError> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err(DowError::new(format!("Invalid version format (need X.Y.Z): {}", version), 1));
    }
    for part in &parts {
        if part.parse::<u32>().is_err() {
            return Err(DowError::new(format!("Invalid version format (non-numeric): {}", version), 1));
        }
    }
    Ok(())
}

fn bump_version(version: &str, bump_type: &str) -> Result<String, DowError> {
    let parts: Vec<u32> = version
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .collect();

    if parts.len() != 3 {
        return Err(DowError::new(format!("Invalid version format: {}", version), 1));
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    match bump_type {
        "major" => Ok(format!("{}.0.0", major + 1)),
        "minor" => Ok(format!("{}.{}.0", major, minor + 1)),
        "patch" => Ok(format!("{}.{}.{}", major, minor, patch + 1)),
        _ => Err(DowError::new(format!("Unknown bump type: {}", bump_type), 1)),
    }
}
