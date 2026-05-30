// dow/src/core/platform.rs
// 平台检测与 XDG 路径约定

use std::env;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let base = env::var("APPDATA")
            .unwrap_or_else(|_| {
                let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
                format!("{}\\AppData\\Roaming", home)
            });
        PathBuf::from(base).join("dow")
    } else {
        let base = env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.config", home)
            });
        PathBuf::from(base).join("dow")
    }
}

pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let base = env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| {
                let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
                format!("{}\\AppData\\Local", home)
            });
        PathBuf::from(base).join("dow")
    } else {
        let base = env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.local/share", home)
            });
        PathBuf::from(base).join("dow")
    }
}

pub fn bin_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        data_dir().join("bin")
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local").join("bin")
    }
}

pub fn bundle_dir() -> PathBuf {
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
        _ => None,
    }
}

pub fn agent_global_instructions(agent: &str) -> Option<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let home = PathBuf::from(home);

    match agent {
        "claude" => Some(home.join(".claude").join("CLAUDE.md")),
        "codex" => Some(home.join(".codex").join("AGENTS.md")),
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
        assert!(agent_plugin_dir("unknown").is_none());
    }
}
