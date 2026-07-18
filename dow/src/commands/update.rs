// dow/src/commands/update.rs
// dow update subcommand — self-update binary + plugins

use std::fs;
use std::path::PathBuf;

use crate::core::{agent_registry, config::DowConfig, github, platform};
use crate::error::DowError;

pub fn run(_human: bool) -> Result<i32, DowError> {
    eprintln!("[dow] Checking for updates...");

    if let Some(method) = detect_install_method() {
        eprintln!("[dow] Detected installation via {}.", method);
        eprintln!("[dow] Please update through your package manager:");
        match method {
            "npm" => eprintln!("  npm update -g @xin_yue/dev-flow"),
            "cargo" => eprintln!("  cargo install dev-flow"),
            _ => {}
        }
        return Ok(0);
    }

    do_update()
}

fn do_update() -> Result<i32, DowError> {
    let current_version = env!("DOW_VERSION");
    let release = github::check_latest_version()
        .map_err(|e| DowError::new(&format!("Update check failed: {}", e), 1))?;

    if github::is_update_available(current_version, &release.version, &release.published_at) {
        eprintln!(
            "[dow] New version found: v{} → v{} ({})",
            current_version, release.version, release.published_at
        );
        if let Some(ref notes) = release.notes {
            eprintln!("[dow] Changes: {}", notes);
        }
    } else {
        eprintln!("[dow] Already at latest version (v{})", current_version);
        return Ok(0);
    }

    let platform_triple = platform::platform_triple();
    let tmp_dir = std::env::temp_dir().join("dow-update");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| DowError::new(&format!("Failed to create temp directory: {}", e), 1))?;

    let tarball_path = tmp_dir.join("dow.tar.gz");
    eprintln!(
        "[dow] Downloading dow-{}-{}...",
        release.tag_name, platform_triple
    );
    github::download_release_asset(&release.tag_name, platform_triple, &tarball_path)
        .map_err(|e| DowError::new(&format!("Download failed: {}", e), 1))?;

    let extract_dir = tmp_dir.join("extracted");
    github::extract_tarball(&tarball_path, &extract_dir)
        .map_err(|e| DowError::new(&format!("Extraction failed: {}", e), 1))?;

    let new_binary = find_binary(&extract_dir)?;
    eprintln!("[dow] Replacing binary...");
    github::self_replace_binary(&new_binary)
        .map_err(|e| DowError::new(&format!("Replacement failed: {}", e), 1))?;

    let new_bundle = extract_dir.join("bundle");
    if new_bundle.exists() {
        let bundle_dest = platform::bundle_dir();
        if bundle_dest.exists() {
            let _ = fs::remove_dir_all(&bundle_dest);
        }
        copy_dir_recursive(&new_bundle, &bundle_dest).map_err(|e| DowError::new(&e, 1))?;
        eprintln!("[dow] Bundle updated");
    }

    sync_agents()?;

    let mut config = DowConfig::load();
    config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
    config.latest_remote_version = None;
    config.latest_remote_published_at = None;
    config.latest_release_notes = None;
    let _ = config.save();

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!(
        "[dow] ✓ Update complete! Current version: v{}",
        release.version
    );
    Ok(0)
}

fn sync_agents() -> Result<(), DowError> {
    let config = DowConfig::load();
    let bundle_dir = platform::bundle_dir();
    for agent in &config.registered_agents {
        match agent_registry::deploy_plugin(agent, &bundle_dir) {
            Ok(()) => eprintln!("[dow]   {} plugin deployed", agent),
            Err(e) => eprintln!("[dow] ⚠ {} plugin deploy failed: {}", agent, e),
        }
        match agent_registry::inject_global_instructions(agent) {
            Ok(true) => eprintln!("[dow]   {} global instructions updated", agent),
            Ok(false) => {}
            Err(e) => eprintln!("[dow] ⚠ {} instructions update failed: {}", agent, e),
        }
        refresh_agent_plugin(agent);
    }
    Ok(())
}

fn detect_install_method() -> Option<&'static str> {
    let exe = std::env::current_exe().ok()?;
    let exe_str = exe.to_string_lossy();
    if exe_str.contains("node_modules") || exe_str.contains("dow-bin") {
        return Some("npm");
    }
    if exe_str.contains(".cargo") {
        return Some("cargo");
    }
    None
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
    let bin_name = if cfg!(target_os = "windows") {
        "dow.exe"
    } else {
        "dow"
    };

    let in_bin = extract_dir.join("bin").join(bin_name);
    if in_bin.exists() {
        return Ok(in_bin);
    }

    let in_root = extract_dir.join(bin_name);
    if in_root.exists() {
        return Ok(in_root);
    }

    Err(DowError::new(
        &format!("Binary {} not found after extraction", bin_name),
        1,
    ))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("Copy failed: {}", e))?;
        }
    }
    Ok(())
}
