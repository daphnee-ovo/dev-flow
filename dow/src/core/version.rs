// dow/src/core/
// ├── version.rs  -- VERSION file multi-branch read/write
//
// Path resolution: VERSION lives at the dev-flow project root resolved by
// doc_root::project_root().
// All read/write goes through doc_root::project_root(); no caller needs to
// thread a path. Path-taking helpers below are private implementation detail.
//
// Format: Each line (<branch>)<version>, e.g.:
//   (main)3.4.0
//   (feature-x)3.4.0
//
// Internal Framework:
// version.rs
// ├── VersionEntry
// ├── resolve_branch() / is_detached()            # branch detection
// ├── read_current                                # public, zero-arg → project_root()
// ├── write_current / write_branch                # public, zero-arg → project_root()
// ├── init_branch / bump                          # public, zero-arg → project_root()
// ├── bump_version_str / bump_version             # pure calc, no file I/O
// ├── read_at / write_at / init_at / bump_at      # private, path-taking
// ├── parse_file(project_root) / write_file(project_root, entries)
// ├── parse_line / is_plain_semver / validate_semver
// └── version_file_path(project_root)
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::core::doc_root;
use crate::error::DowError;
use std::fs;
use std::path::{Path, PathBuf};

struct VersionEntry {
    branch: String,
    version: String,
}

/// Resolve VERSION file path under a project root.
fn version_file_path(project_root: &Path) -> PathBuf {
    project_root.join("VERSION")
}

/// Read current branch version number. Falls back to 'main' on detached HEAD.
pub fn read_current() -> Result<String, DowError> {
    let project_root = doc_root::project_root();
    read_at(&project_root, &resolve_branch())
}

/// Resolve branch: current branch or fallback to 'main'
pub fn resolve_branch() -> String {
    doc_root::current_branch().unwrap_or_else(|| "main".to_string())
}

/// Whether the current state is a detached HEAD (no branch detected)
pub fn is_detached() -> bool {
    doc_root::current_branch().is_none()
}

/// Write current branch version number (update existing line or append new line).
pub fn write_current(version: &str) -> Result<(), DowError> {
    let branch = doc_root::current_branch()
        .ok_or_else(|| DowError::new("Failed to get current branch", 1))?;
    write_at(&doc_root::project_root(), &branch, version)
}

/// Write specified branch version number.
pub fn write_branch(branch: &str, version: &str) -> Result<(), DowError> {
    write_at(&doc_root::project_root(), branch, version)
}

/// Initialize version for new branch (inherit from main).
pub fn init_branch(branch: &str) -> Result<String, DowError> {
    init_at(&doc_root::project_root(), branch)
}

/// Bump current branch version number (read + calculate + write).
pub fn bump(bump_type: &str) -> Result<(String, String), DowError> {
    bump_at(&doc_root::project_root(), bump_type)
}

/// Pure calculation: given version number + bump type, return new version number (don't write to file)
pub fn bump_version_str(version: &str, bump_type: &str) -> Result<String, DowError> {
    bump_version(version, bump_type)
}

// ─── private, path-taking implementation ─────────────────────────────────────

fn read_at(project_root: &Path, branch: &str) -> Result<String, DowError> {
    let entries = parse_file(project_root)?;
    entries
        .iter()
        .find(|e| e.branch == branch)
        .map(|e| e.version.clone())
        .ok_or_else(|| DowError::new(format!("No record for branch {} in VERSION", branch), 1))
}

fn write_at(project_root: &Path, branch: &str, version: &str) -> Result<(), DowError> {
    validate_semver(version)?;
    let mut entries = parse_file(project_root).unwrap_or_default();

    if let Some(entry) = entries.iter_mut().find(|e| e.branch == branch) {
        entry.version = version.to_string();
    } else {
        entries.push(VersionEntry {
            branch: branch.to_string(),
            version: version.to_string(),
        });
    }

    write_file(project_root, &entries)
}

fn init_at(project_root: &Path, branch: &str) -> Result<String, DowError> {
    let entries = parse_file(project_root).unwrap_or_default();

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

    write_at(project_root, branch, &base_version)?;
    Ok(base_version)
}

fn bump_at(project_root: &Path, bump_type: &str) -> Result<(String, String), DowError> {
    let current = read_at(project_root, &resolve_branch())?;
    let new = bump_version(&current, bump_type)?;
    let branch = doc_root::current_branch()
        .ok_or_else(|| DowError::new("Failed to get current branch", 1))?;
    write_at(project_root, &branch, &new)?;
    Ok((current, new))
}

/// Parse VERSION file under project_root, compatible with old format
/// (plain version number, treated as main)
fn parse_file(project_root: &Path) -> Result<Vec<VersionEntry>, DowError> {
    let path = version_file_path(project_root);
    let content = fs::read_to_string(&path).map_err(|_| {
        DowError::new(
            format!(
                "VERSION file does not exist or is not readable at {}",
                path.display()
            ),
            1,
        )
    })?;

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
        return Err(DowError::new(
            "VERSION file is empty or format cannot be parsed",
            1,
        ));
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

fn write_file(project_root: &Path, entries: &[VersionEntry]) -> Result<(), DowError> {
    let content: String = entries
        .iter()
        .map(|e| format!("({}){}", e.branch, e.version))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    fs::write(version_file_path(project_root), content).map_err(|e| DowError::new(e.to_string(), 1))
}

fn validate_semver(version: &str) -> Result<(), DowError> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err(DowError::new(
            format!("Invalid version format (need X.Y.Z): {}", version),
            1,
        ));
    }
    for part in &parts {
        if part.parse::<u32>().is_err() {
            return Err(DowError::new(
                format!("Invalid version format (non-numeric): {}", version),
                1,
            ));
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
        return Err(DowError::new(
            format!("Invalid version format: {}", version),
            1,
        ));
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    match bump_type {
        "major" => Ok(format!("{}.0.0", major + 1)),
        "minor" => Ok(format!("{}.{}.0", major, minor + 1)),
        "patch" => Ok(format!("{}.{}.{}", major, minor, patch + 1)),
        _ => Err(DowError::new(
            format!("Unknown bump type: {}", bump_type),
            1,
        )),
    }
}
