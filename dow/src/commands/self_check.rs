// dow/src/commands/self_check.rs
// dow self-check 子命令 — 安装状态诊断

use crate::core::{agent_registry, config::DowConfig, platform};
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

    // 版本检查状态
    if let Some(ref last) = config.last_version_check {
        eprintln!("上次版本检查: {}", last);
    } else {
        eprintln!("上次版本检查: 从未");
    }
    if let Some(ref remote) = config.latest_remote_version {
        eprintln!("远程最新版本: v{}", remote);
    }

    Ok(0)
}
