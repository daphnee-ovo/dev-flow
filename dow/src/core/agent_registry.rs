// dow/src/core/agent_registry.rs
// Agent plugin directory discovery and file deployment
//
// Internal Framework:
// agent_registry.rs
// ├── deploy_plugin()
// ├── inject_global_instructions()
// │   └── build_global_instruction_content()
// ├── verify_plugin_integrity()
// └── copy_dir_recursive()
//
// Related Docs:
// - [Agent Instructions](../../../AGENTS.md#dev-flow)

use std::fs;
use std::path::Path;

use super::platform;

const DEV_FLOW_TAG_OPEN: &str = "<dev-flow>";
const DEV_FLOW_TAG_CLOSE: &str = "</dev-flow>";
const CODEX_DISCIPLINE_PLACEHOLDER: &str = "{CODEX DEV FLOW Discipline}";

pub struct AgentInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}

pub const SUPPORTED_AGENTS: &[AgentInfo] = &[
    AgentInfo {
        name: "claude",
        display_name: "Claude Code",
    },
    AgentInfo {
        name: "codex",
        display_name: "Codex",
    },
    AgentInfo {
        name: "kiro",
        display_name: "Kiro",
    },
];

pub fn deploy_plugin(agent: &str, bundle_dir: &Path) -> Result<(), String> {
    let source = bundle_dir.join(agent);
    if !source.exists() {
        return Err(format!(
            "Plugin resource for {} not found in bundle: {}",
            agent,
            source.display()
        ));
    }

    let target =
        platform::agent_plugin_dir(agent).ok_or_else(|| format!("Unsupported agent: {}", agent))?;

    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to clean old plugin directory: {}", e))?;
    }

    copy_dir_recursive(&source, &target)?;

    Ok(())
}

pub fn inject_global_instructions(agent: &str) -> Result<bool, String> {
    let instructions_path = platform::agent_global_instructions(agent)
        .ok_or_else(|| format!("Unsupported agent: {}", agent))?;

    let content = if instructions_path.exists() {
        fs::read_to_string(&instructions_path)
            .map_err(|e| format!("Failed to read global instructions file: {}", e))?
    } else {
        String::new()
    };

    let (new_content, changed) = build_global_instruction_content(agent, &content);
    if !changed {
        return Ok(false);
    }

    if let Some(parent) = instructions_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(&instructions_path, new_content)
        .map_err(|e| format!("Failed to write global instructions file: {}", e))?;

    Ok(true)
}

fn build_global_instruction_content(agent: &str, content: &str) -> (String, bool) {
    let block = build_dev_flow_block(agent);
    let new_content = replace_or_append_block(content, &block);
    let changed = new_content != content;
    (new_content, changed)
}

fn build_dev_flow_block(agent: &str) -> String {
    let template = include_str!("../../references/inject_prompt/dev_flow.md");
    let discipline = if agent == "codex" {
        include_str!("../../references/inject_prompt/codex_hook_discipline.md").to_string()
    } else {
        String::new()
    };
    template.replace(CODEX_DISCIPLINE_PLACEHOLDER, &discipline)
}

fn replace_or_append_block(content: &str, block: &str) -> String {
    if let (Some(start), Some(end_tag_start)) = (
        content.find(DEV_FLOW_TAG_OPEN),
        content.find(DEV_FLOW_TAG_CLOSE),
    ) {
        let end = end_tag_start + DEV_FLOW_TAG_CLOSE.len();
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..start]);
        result.push_str(block);
        result.push_str(&content[end..]);
        result
    } else {
        let mut result = content.to_string();
        result.push_str("\n\n");
        result.push_str(block);
        result
    }
}

pub fn verify_plugin_integrity(agent: &str) -> Result<Vec<String>, String> {
    let target =
        platform::agent_plugin_dir(agent).ok_or_else(|| format!("Unsupported agent: {}", agent))?;

    let mut issues = Vec::new();

    if !target.exists() {
        issues.push(format!(
            "Plugin directory does not exist: {}",
            target.display()
        ));
        return Ok(issues);
    }

    let required_dirs: &[&str] = match agent {
        "claude" => &["commands", "agents"],
        "codex" => &["skills", "agents"],
        _ => &[],
    };
    for dir in required_dirs {
        if !target.join(dir).exists() {
            issues.push(format!("Missing directory: {}", target.join(dir).display()));
        }
    }

    match agent {
        "claude" => {
            if !target.join("hooks").join("hooks.json").exists() {
                issues.push("Missing hooks/hooks.json".to_string());
            }
            if !target.join(".claude-plugin").join("plugin.json").exists() {
                issues.push("Missing .claude-plugin/plugin.json".to_string());
            }
        }
        "codex" => {
            if !target.join("hooks").join("hooks.json").exists() {
                issues.push("Missing hooks/hooks.json".to_string());
            }
            if !target.join(".app.json").exists() {
                issues.push("Missing .app.json".to_string());
            }
            if !target
                .join(".agents")
                .join("plugins")
                .join("marketplace.json")
                .exists()
            {
                issues.push("Missing .agents/plugins/marketplace.json".to_string());
            }
            if !target.join(".codex-plugin").join("plugin.json").exists() {
                issues.push("Missing .codex-plugin/plugin.json".to_string());
            }
        }
        _ => {}
    }

    Ok(issues)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "Failed to copy {} → {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}
