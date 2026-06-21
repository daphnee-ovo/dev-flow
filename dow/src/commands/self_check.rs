// dow/src/commands/self_check.rs
// dow self-check 子命令 — 安装状态诊断

use crate::core::{agent_registry, config::DowConfig, github, platform};
use crate::error::DowError;

pub fn run(_human: bool) -> Result<i32, DowError> {
    let config = DowConfig::load();
    let current_version = env!("DOW_VERSION");

    eprintln!("=== dow self-check ===\n");
    eprintln!("版本: v{}", current_version);
    eprintln!("平台: {}", platform::platform_triple());
    eprintln!("配置: {}", DowConfig::path().display());
    eprintln!("Bundle: {}", platform::bundle_dir().display());
    eprintln!();

    // 路径检查
    let config_exists = DowConfig::path().exists();
    let bundle_exists = platform::bundle_dir().exists();
    eprintln!("路径状态:");
    eprintln!("  配置文件: {}", if config_exists { "✓" } else { "✗ 不存在" });
    eprintln!("  Bundle: {}", if bundle_exists { "✓" } else { "✗ 不存在" });
    eprintln!();

    // 已注册 agent
    eprintln!("已注册 agent ({}):", config.registered_agents.len());
    if config.registered_agents.is_empty() {
        eprintln!("  （无，运行 `dow setup` 注册）");
    } else {
        for agent in &config.registered_agents {
            let display = agent_registry::SUPPORTED_AGENTS
                .iter()
                .find(|a| a.name == agent)
                .map(|a| a.display_name)
                .unwrap_or(agent.as_str());

            let issues = agent_registry::verify_plugin_integrity(agent)
                .unwrap_or_default();

            if issues.is_empty() {
                eprintln!("  ✓ {} — 完整", display);
            } else {
                eprintln!("  ⚠ {} — {} 项问题:", display, issues.len());
                for issue in &issues {
                    eprintln!("      - {}", issue);
                }
            }
        }
    }
    eprintln!();

    // 实时获取远程最新版本
    eprint!("远程最新版本: ");
    match github::check_latest_version() {
        Ok(release) => {
            eprintln!("v{} ({})", release.version, release.published_at);
            if github::is_update_available(current_version, &release.version, &release.published_at)
            {
                eprintln!("  → 有新版本可用，运行 `dow update` 升级");
            } else {
                match github::compare_versions(current_version, &release.version) {
                    std::cmp::Ordering::Equal => {
                        eprintln!("  → 已是最新");
                    }
                    std::cmp::Ordering::Greater => {
                        eprintln!("  → 本地版本超前（开发中）");
                    }
                    std::cmp::Ordering::Less => {
                        eprintln!("  → 远程版本日期无效，跳过更新提示");
                    }
                }
            }
            // 更新缓存
            let mut config = config;
            config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
            config.latest_remote_version = Some(release.version);
            config.latest_remote_published_at = Some(release.published_at);
            config.latest_release_notes = release.notes;
            let _ = config.save();
        }
        Err(e) => {
            eprintln!("查询失败 ({})", e);
            if let (Some(cached), Some(published_at)) = (
                config.latest_remote_version.as_deref(),
                config.latest_remote_published_at.as_deref(),
            ) {
                eprintln!("  缓存版本: v{} ({})", cached, published_at);
            }
        }
    }

    Ok(0)
}
