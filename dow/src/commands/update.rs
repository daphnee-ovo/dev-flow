// dow/src/commands/update.rs
// dow update 子命令 — 自更新二进制 + 插件

use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

use crate::core::{agent_registry, config::DowConfig, github, platform};
use crate::error::DowError;

pub fn run(_human: bool) -> Result<i32, DowError> {
    eprintln!("[dow] 检查更新...");

    let current_version = env!("DOW_VERSION");
    let release = github::check_latest_version()
        .map_err(|e| DowError::new(&format!("检查更新失败: {}", e), 1))?;

    match github::compare_versions(current_version, &release.version) {
        Ordering::Less => {
            eprintln!("[dow] 发现新版本: v{} → v{}", current_version, release.version);
            if let Some(ref notes) = release.notes {
                eprintln!("[dow] 变更: {}", notes);
            }
        }
        _ => {
            eprintln!("[dow] 已是最新版本 (v{})", current_version);
            return Ok(0);
        }
    }

    let platform_triple = platform::platform_triple();
    let tmp_dir = std::env::temp_dir().join("dow-update");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| DowError::new(&format!("创建临时目录失败: {}", e), 1))?;

    let tarball_path = tmp_dir.join("dow.tar.gz");
    eprintln!("[dow] 下载 dow-{}-{}...", release.tag_name, platform_triple);
    github::download_release_asset(&release.tag_name, platform_triple, &tarball_path)
        .map_err(|e| DowError::new(&format!("下载失败: {}", e), 1))?;

    let extract_dir = tmp_dir.join("extracted");
    github::extract_tarball(&tarball_path, &extract_dir)
        .map_err(|e| DowError::new(&format!("解压失败: {}", e), 1))?;

    let new_binary = find_binary(&extract_dir)?;
    eprintln!("[dow] 替换二进制...");
    github::self_replace_binary(&new_binary)
        .map_err(|e| DowError::new(&format!("替换失败: {}", e), 1))?;

    let new_bundle = extract_dir.join("bundle");
    if new_bundle.exists() {
        let bundle_dest = platform::bundle_dir();
        if bundle_dest.exists() {
            let _ = fs::remove_dir_all(&bundle_dest);
        }
        copy_dir_recursive(&new_bundle, &bundle_dest)
            .map_err(|e| DowError::new(&e, 1))?;
        eprintln!("[dow] bundle 已更新");
    }

    let config = DowConfig::load();
    let bundle_dir = platform::bundle_dir();
    for agent in &config.registered_agents {
        match agent_registry::deploy_plugin(agent, &bundle_dir) {
            Ok(()) => {
                refresh_agent_plugin(agent);
                eprintln!("[dow] ✓ {} 插件已更新", agent);
            }
            Err(e) => eprintln!("[dow] ⚠ {} 插件更新失败: {}", agent, e),
        }
    }

    let mut config = DowConfig::load();
    config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
    config.latest_remote_version = None;
    let _ = config.save();

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!("[dow] ✓ 更新完成！当前版本: v{}", release.version);
    Ok(0)
}

fn refresh_agent_plugin(agent: &str) {
    match agent {
        "claude" => {
            let _ = std::process::Command::new("claude")
                .args(["plugin", "update", "dev-flow@dev-flow"])
                .output();
        }
        _ => {}
    }
}

fn find_binary(extract_dir: &std::path::Path) -> Result<PathBuf, DowError> {
    let bin_name = if cfg!(target_os = "windows") { "dow.exe" } else { "dow" };

    let in_bin = extract_dir.join("bin").join(bin_name);
    if in_bin.exists() {
        return Ok(in_bin);
    }

    let in_root = extract_dir.join(bin_name);
    if in_root.exists() {
        return Ok(in_root);
    }

    Err(DowError::new(&format!("解压后未找到 {} 二进制", bin_name), 1))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("读取目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("复制失败: {}", e))?;
        }
    }
    Ok(())
}
