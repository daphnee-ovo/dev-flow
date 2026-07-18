// dow/src/commands/self_check.rs
// dow self-check subcommand — installation status diagnostics

use crate::core::{agent_registry, config::DowConfig, github, platform};
use crate::error::DowError;

pub fn run(_human: bool) -> Result<i32, DowError> {
    let config = DowConfig::load();
    let current_version = env!("DOW_VERSION");

    eprintln!("=== dow self-check ===\n");
    eprintln!("Version: v{}", current_version);
    eprintln!("Platform: {}", platform::platform_triple());
    eprintln!("Config: {}", DowConfig::path().display());
    eprintln!("Bundle: {}", platform::bundle_dir().display());
    eprintln!();

    // Path check
    let config_exists = DowConfig::path().exists();
    let bundle_exists = platform::bundle_dir().exists();
    eprintln!("Path status:");
    eprintln!(
        "  Config file: {}",
        if config_exists {
            "✓"
        } else {
            "✗ does not exist"
        }
    );
    eprintln!(
        "  Bundle: {}",
        if bundle_exists {
            "✓"
        } else {
            "✗ does not exist"
        }
    );
    eprintln!();

    // Registered agents
    eprintln!("Registered agents ({}):", config.registered_agents.len());
    if config.registered_agents.is_empty() {
        eprintln!("  (none, run `dow setup` to register)");
    } else {
        for agent in &config.registered_agents {
            let display = agent_registry::SUPPORTED_AGENTS
                .iter()
                .find(|a| a.name == agent)
                .map(|a| a.display_name)
                .unwrap_or(agent.as_str());

            let issues = agent_registry::verify_plugin_integrity(agent).unwrap_or_default();

            if issues.is_empty() {
                eprintln!("  ✓ {} — complete", display);
            } else {
                eprintln!("  ⚠ {} — {} issues:", display, issues.len());
                for issue in &issues {
                    eprintln!("      - {}", issue);
                }
            }
        }
    }
    eprintln!();

    // Fetch latest remote version in real-time
    eprint!("Latest remote version: ");
    match github::check_latest_version() {
        Ok(release) => {
            eprintln!("v{} ({})", release.version, release.published_at);
            if github::is_update_available(current_version, &release.version, &release.published_at)
            {
                eprintln!("  → New version available, run `dow update` to upgrade");
            } else {
                match github::compare_versions(current_version, &release.version) {
                    std::cmp::Ordering::Equal => {
                        eprintln!("  → Already up to date");
                    }
                    std::cmp::Ordering::Greater => {
                        eprintln!("  → Local version is ahead (in development)");
                    }
                    std::cmp::Ordering::Less => {
                        eprintln!("  → Remote version date invalid, skipping update prompt");
                    }
                }
            }
            // Update cache
            let mut config = config;
            config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
            config.latest_remote_version = Some(release.version);
            config.latest_remote_published_at = Some(release.published_at);
            config.latest_release_notes = release.notes;
            let _ = config.save();
        }
        Err(e) => {
            eprintln!("Query failed ({})", e);
            if let (Some(cached), Some(published_at)) = (
                config.latest_remote_version.as_deref(),
                config.latest_remote_published_at.as_deref(),
            ) {
                eprintln!("  Cached version: v{} ({})", cached, published_at);
            }
        }
    }

    Ok(0)
}
