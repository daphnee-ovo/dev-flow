// dow/src/core/
// ├── version.rs  -- VERSION 文件多分支读写
//
// 格式：每行 (<branch>)<version>，如：
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

/// 读取当前分支的版本号
pub fn read_current() -> Result<String, DowError> {
    let branch = doc_root::current_branch()
        .ok_or_else(|| DowError::new("无法获取当前分支", 1))?;
    read_branch(&branch)
}

/// 读取指定分支的版本号
pub fn read_branch(branch: &str) -> Result<String, DowError> {
    let entries = parse_file()?;
    entries
        .iter()
        .find(|e| e.branch == branch)
        .map(|e| e.version.clone())
        .ok_or_else(|| DowError::new(format!("VERSION 中无分支 {} 的记录", branch), 1))
}

/// 写入当前分支的版本号（更新已有行或追加新行）
pub fn write_current(version: &str) -> Result<(), DowError> {
    let branch = doc_root::current_branch()
        .ok_or_else(|| DowError::new("无法获取当前分支", 1))?;
    write_branch(&branch, version)
}

/// 写入指定分支的版本号
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

/// 删除指定分支的版本记录
pub fn remove_branch(branch: &str) -> Result<(), DowError> {
    let mut entries = parse_file().unwrap_or_default();
    entries.retain(|e| e.branch != branch);
    write_file(&entries)
}

/// 为新分支初始化版本（继承 main 的版本号）
pub fn init_branch(branch: &str) -> Result<String, DowError> {
    let entries = parse_file().unwrap_or_default();

    // 已存在则直接返回
    if let Some(entry) = entries.iter().find(|e| e.branch == branch) {
        return Ok(entry.version.clone());
    }

    // 继承 main/master 的版本
    let base_version = entries
        .iter()
        .find(|e| e.branch == "main" || e.branch == "master")
        .map(|e| e.version.clone())
        .unwrap_or_else(|| "0.1.0".to_string());

    write_branch(branch, &base_version)?;
    Ok(base_version)
}

/// bump 当前分支版本号（读取 + 计算 + 写入）
pub fn bump(bump_type: &str) -> Result<(String, String), DowError> {
    let current = read_current()?;
    let new = bump_version(&current, bump_type)?;
    write_current(&new)?;
    Ok((current, new))
}

/// 纯计算：给定版本号 + bump 类型，返回新版本号（不写入文件）
pub fn bump_version_str(version: &str, bump_type: &str) -> Result<String, DowError> {
    bump_version(version, bump_type)
}

/// 解析版本文件，兼容旧格式（纯版本号，视为 main）
fn parse_file() -> Result<Vec<VersionEntry>, DowError> {
    let content = fs::read_to_string("VERSION")
        .map_err(|_| DowError::new("VERSION 文件不存在或不可读", 1))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(entry) = parse_line(trimmed) {
            entries.push(entry);
        } else if is_plain_semver(trimmed) {
            // 旧格式兼容：纯版本号视为 main
            entries.push(VersionEntry {
                branch: "main".to_string(),
                version: trimmed.to_string(),
            });
        }
    }

    if entries.is_empty() {
        return Err(DowError::new("VERSION 文件为空或格式无法解析", 1));
    }

    Ok(entries)
}

/// 解析单行：(branch)version
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
        return Err(DowError::new(format!("版本格式非法（需要 X.Y.Z）：{}", version), 1));
    }
    for part in &parts {
        if part.parse::<u32>().is_err() {
            return Err(DowError::new(format!("版本格式非法（非数字）：{}", version), 1));
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
        return Err(DowError::new(format!("版本格式非法：{}", version), 1));
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    match bump_type {
        "major" => Ok(format!("{}.0.0", major + 1)),
        "minor" => Ok(format!("{}.{}.0", major, minor + 1)),
        "patch" => Ok(format!("{}.{}.{}", major, minor, patch + 1)),
        _ => Err(DowError::new(format!("未知 bump 类型：{}", bump_type), 1)),
    }
}
