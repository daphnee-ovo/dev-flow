// dow/src/core/config.rs
// Global configuration (~/.config/dow/config.toml) read/write
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::platform;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DowConfig {
    #[serde(default)]
    pub registered_agents: Vec<String>,
    #[serde(default)]
    pub last_version_check: Option<String>,
    #[serde(default)]
    pub latest_remote_version: Option<String>,
    #[serde(default)]
    pub latest_remote_published_at: Option<String>,
    #[serde(default)]
    pub latest_release_notes: Option<String>,
}

impl DowConfig {
    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    pub fn path() -> PathBuf {
        platform::config_dir().join("config.toml")
    }

    pub fn add_agent(&mut self, agent: &str) {
        if !self.registered_agents.contains(&agent.to_string()) {
            self.registered_agents.push(agent.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DowConfig::default();
        assert!(config.registered_agents.is_empty());
        assert!(config.last_version_check.is_none());
    }

    #[test]
    fn test_add_agent() {
        let mut config = DowConfig::default();
        config.add_agent("claude");
        assert!(config.registered_agents.contains(&"claude".to_string()));
        assert!(!config.registered_agents.contains(&"codex".to_string()));
        config.add_agent("claude");
        assert_eq!(config.registered_agents.len(), 1);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut config = DowConfig::default();
        config.add_agent("claude");
        config.last_version_check = Some("2026-05-30T12:00:00Z".to_string());
        config.latest_remote_version = Some("0.1.5".to_string());
        config.latest_remote_published_at = Some("2026-06-18T10:42:41Z".to_string());
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: DowConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.registered_agents, vec!["claude"]);
        assert_eq!(
            deserialized.last_version_check,
            Some("2026-05-30T12:00:00Z".to_string())
        );
        assert_eq!(
            deserialized.latest_remote_version,
            Some("0.1.5".to_string())
        );
        assert_eq!(
            deserialized.latest_remote_published_at,
            Some("2026-06-18T10:42:41Z".to_string())
        );
    }
}
