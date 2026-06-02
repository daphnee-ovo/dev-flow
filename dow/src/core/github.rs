// dow/src/core/github.rs
// GitHub Release API 交互（版本检查、下载）
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use std::fs;
use std::path::Path;
use std::process::Command;

const GITHUB_REPO: &str = "daphnee-ovo/dev-flow";
const API_BASE: &str = "https://api.github.com";

#[derive(Debug)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub notes: Option<String>,
}

fn resolve_github_token() -> Option<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()?;
    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
}

pub fn check_latest_version() -> Result<ReleaseInfo, String> {
    let url = format!("{}/repos/{}/releases/latest", API_BASE, GITHUB_REPO);

    let client = reqwest::blocking::Client::builder()
        .user_agent("dow-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let mut request = client.get(&url);
    if let Some(token) = resolve_github_token() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let resp = request
        .send()
        .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回 {}", resp.status()));
    }

    let body: serde_json::Value = resp.json()
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let tag_name = body["tag_name"]
        .as_str()
        .ok_or("响应中无 tag_name")?
        .to_string();

    let version = tag_name.trim_start_matches('v').to_string();

    let notes = body["body"].as_str().map(|s| truncate_notes(s, 3));

    Ok(ReleaseInfo { tag_name, version, notes })
}

fn truncate_notes(body: &str, max_lines: usize) -> String {
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn download_release_asset(tag: &str, platform: &str, dest: &Path) -> Result<(), String> {
    let filename = format!("dow-{}-{}.tar.gz", tag, platform);
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, tag, filename
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("dow-cli")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let resp = client.get(&url)
        .send()
        .map_err(|e| format!("下载失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("下载失败: HTTP {}", resp.status()));
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let bytes = resp.bytes()
        .map_err(|e| format!("读取响应体失败: {}", e))?;
    fs::write(dest, &bytes)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}

pub fn extract_tarball(tarball_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(tarball_path)
        .map_err(|e| format!("打开 tarball 失败: {}", e))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("创建目标目录失败: {}", e))?;

    archive.unpack(dest_dir)
        .map_err(|e| format!("解压失败: {}", e))?;

    Ok(())
}

pub fn compare_versions(current: &str, remote: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let a = parse(current);
    let b = parse(remote);
    a.cmp(&b)
}

pub fn self_replace_binary(new_binary: &Path) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("无法获取当前二进制路径: {}", e))?;

    let backup = current_exe.with_extension("old");
    if current_exe.exists() {
        fs::rename(&current_exe, &backup)
            .map_err(|e| format!("备份旧二进制失败: {}", e))?;
    }

    fs::copy(new_binary, &current_exe)
        .map_err(|e| {
            // 恢复备份
            let _ = fs::rename(&backup, &current_exe);
            format!("替换二进制失败: {}", e)
        })?;

    let _ = fs::remove_file(&backup);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("3.8.3", "3.8.3"), Ordering::Equal);
        assert_eq!(compare_versions("3.8.3", "3.9.0"), Ordering::Less);
        assert_eq!(compare_versions("4.0.0", "3.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("v3.8.3", "3.8.3"), Ordering::Equal);
    }
}
