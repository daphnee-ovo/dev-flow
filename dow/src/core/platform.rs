// dow/src/core/platform.rs
// Platform detection and XDG path conventions
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use std::env;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let base = env::var("APPDATA").unwrap_or_else(|_| {
                let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
                format!("{}\\AppData\\Roaming", home)
            });
        PathBuf::from(base).join("dow")
    } else {
        let base = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.config", home)
            });
        PathBuf::from(base).join("dow")
    }
}

pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let base = env::var("LOCALAPPDATA").unwrap_or_else(|_| {
                let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
                format!("{}\\AppData\\Local", home)
            });
        PathBuf::from(base).join("dow")
    } else {
        let base = env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.local/share", home)
            });
        PathBuf::from(base).join("dow")
    }
}

pub fn bundle_dir() -> PathBuf {
    // Prioritize checking bundle adjacent to exe (supports winget portable / manual extraction)
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // tar.gz structure: ./bin/dow.exe + ./bundle/
            let adjacent = exe_dir.parent().unwrap_or(exe_dir).join("bundle");
            if adjacent.is_dir() {
                return adjacent;
            }
            // Sibling: dow.exe + bundle/ (zip scenario)
            let sibling = exe_dir.join("bundle");
            if sibling.is_dir() {
                return sibling;
            }
        }
    }
    data_dir().join("bundle")
}

pub fn agent_plugin_dir(agent: &str) -> Option<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let home = PathBuf::from(home);

    match agent {
        "claude" => Some(home.join(".claude").join("plugins").join("dev-flow")),
        "codex" => Some(home.join(".codex").join("plugins").join("dev-flow")),
        "kiro" => Some(home.join(".kiro").join("skills")),
        "pi" => Some(home.join(".pi").join("agent").join("extensions").join("dev-flow")),
        _ => None,
    }
}

/// Candidate Codex binaries shipped inside the macOS Codex/ChatGPT apps.
///
/// The standalone CLI and the app-integrated runtime expose the same plugin
/// subcommands, but the latter is not necessarily added to the user's PATH.
pub fn codex_cli_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let mut app_roots = vec![PathBuf::from("/Applications")];
        if let Ok(home) = env::var("HOME") {
            app_roots.push(PathBuf::from(home).join("Applications"));
        }

        for root in app_roots {
            for app in ["ChatGPT.app", "Codex.app"] {
                let app_root = root.join(app);
                candidates.push(app_root.join("Contents").join("Resources").join("codex"));
                candidates.push(app_root.join("Contents").join("MacOS").join("codex"));
                candidates.push(app_root.join("Contents").join("MacOS").join("Codex"));
            }
        }
    }

    candidates
}

pub fn codex_personal_marketplace_dir() -> Option<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Some(
        PathBuf::from(home)
            .join(".codex")
            .join("plugins")
            .join(".agents")
            .join("plugins"),
    )
}

pub fn agent_global_instructions(agent: &str) -> Option<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let home = PathBuf::from(home);

    match agent {
        "claude" => Some(home.join(".claude").join("CLAUDE.md")),
        "codex" => Some(home.join(".codex").join("AGENTS.md")),
        "kiro" => Some(home.join(".kiro").join("steering").join("steering.md")),
        "pi" => Some(home.join(".pi").join("agent").join("AGENTS.md")),
        _ => None,
    }
}

pub fn platform_triple() -> &'static str {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "linux-x86_64"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "linux-aarch64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "darwin-arm64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "darwin-x86_64"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "windows-x86_64"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_triple_not_unknown() {
        let triple = platform_triple();
        assert_ne!(triple, "unknown");
    }

    #[test]
    fn test_config_dir_not_empty() {
        let dir = config_dir();
        assert!(dir.to_str().unwrap().contains("dow"));
    }

    #[test]
    fn test_agent_plugin_dir() {
        assert!(agent_plugin_dir("claude").is_some());
        assert!(agent_plugin_dir("codex").is_some());
        assert!(agent_plugin_dir("pi").is_some());
        assert!(agent_plugin_dir("unknown").is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_codex_cli_candidates_include_chatgpt_app_runtime() {
        assert!(codex_cli_candidates()
            .iter()
            .any(|path| { path.ends_with(PathBuf::from("ChatGPT.app/Contents/Resources/codex")) }));
    }
}
