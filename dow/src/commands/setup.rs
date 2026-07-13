// dow/src/commands/setup.rs
// dow setup subcommand — TUI interaction + agent registration

use dialoguer::MultiSelect;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{agent_registry, config::DowConfig, platform};
use crate::error::DowError;

const CODEX_MARKETPLACE_NAME: &str = "dev-flow-local";
const CODEX_PLUGIN_NAME: &str = "dev-flow";

pub fn run(agent: Option<String>, _human: bool) -> Result<i32, DowError> {
    let agents = resolve_agents(agent)?;

    let bundle = platform::bundle_dir();
    if !bundle.exists() {
        return Err(DowError::new(
            "Plugin bundle does not exist. Please run the installation script first or check the ~/.local/share/dow/bundle/ directory.",
            1,
        ));
    }

    let mut config = DowConfig::load();

    for agent_name in &agents {
        eprint!("[dow] Registering to {}...", agent_display_name(agent_name));

        agent_registry::deploy_plugin(agent_name, &bundle).map_err(|e| {
            DowError::new(&format!("Failed to deploy {} plugin: {}", agent_name, e), 1)
        })?;

        match agent_registry::inject_global_instructions(agent_name) {
            Ok(true) => eprintln!(" Global instructions injected"),
            Ok(false) => eprintln!(" Global instructions already exist, skipped"),
            Err(e) => eprintln!(" Failed to inject global instructions: {}", e),
        }

        register_with_agent(agent_name).map_err(|e| {
            DowError::new(
                &format!(
                    "{} registration failed: {}",
                    agent_display_name(agent_name),
                    e
                ),
                1,
            )
        })?;

        config.add_agent(agent_name);
        eprintln!(
            "[dow] ✓ {} registration completed",
            agent_display_name(agent_name)
        );
    }

    config.save().map_err(|e| DowError::new(&e, 1))?;

    eprintln!("\n[dow] Done! Run /init in your project to start using dev-flow.");
    Ok(0)
}

fn resolve_agents(agent_arg: Option<String>) -> Result<Vec<String>, DowError> {
    match agent_arg.as_deref() {
        Some("all") => Ok(agent_registry::SUPPORTED_AGENTS
            .iter()
            .map(|a| a.name.to_string())
            .collect()),
        Some(name) => {
            if agent_registry::SUPPORTED_AGENTS
                .iter()
                .any(|a| a.name == name)
            {
                Ok(vec![name.to_string()])
            } else {
                Err(DowError::new(
                    &format!(
                        "Unsupported agent: {} (options: claude, codex, kiro, all)",
                        name
                    ),
                    1,
                ))
            }
        }
        None => interactive_select(),
    }
}

fn interactive_select() -> Result<Vec<String>, DowError> {
    let mut items: Vec<&str> = agent_registry::SUPPORTED_AGENTS
        .iter()
        .map(|a| a.display_name)
        .collect();
    items.push("All");

    let defaults: Vec<bool> = items.iter().map(|_| false).collect();

    let selections = MultiSelect::new()
        .with_prompt("Select agents to register (Space to select, Enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .map_err(|e| DowError::new(&format!("Interactive selection failed: {}", e), 1))?;

    if selections.is_empty() {
        return Err(DowError::new("No agent selected", 1));
    }

    let all_index = items.len() - 1;
    if selections.contains(&all_index) {
        return Ok(agent_registry::SUPPORTED_AGENTS
            .iter()
            .map(|a| a.name.to_string())
            .collect());
    }

    Ok(selections
        .iter()
        .map(|&i| agent_registry::SUPPORTED_AGENTS[i].name.to_string())
        .collect())
}

fn register_with_agent(agent: &str) -> Result<(), String> {
    match agent {
        "claude" => {
            if let Some(plugin_dir) = platform::agent_plugin_dir("claude") {
                let path_str = plugin_dir.to_str().unwrap_or("");

                // Register/update marketplace source
                let _ = std::process::Command::new("claude")
                    .args(["plugin", "marketplace", "add", path_str])
                    .output();

                // Try update; if not installed, fall back to install
                // Try update; if not installed, fall back to install
                let update = std::process::Command::new("claude")
                    .args(["plugin", "update", "dev-flow@dev-flow"])
                    .output();

                let need_install = match &update {
                    Ok(o) => !o.status.success(),
                    Err(_) => true,
                };

                if need_install {
                    let _ = std::process::Command::new("claude")
                        .args(["plugin", "install", "dev-flow"])
                        .output();
                }
            }
            Ok(())
        }
        "codex" => platform::agent_plugin_dir("codex")
            .map(|plugin_dir| register_codex_plugin(&plugin_dir))
            .unwrap_or_else(|| Err("Cannot determine Codex plugin directory".to_string())),
        "kiro" => platform::agent_plugin_dir("kiro")
            .map(|plugin_dir| register_kiro_plugin(&plugin_dir))
            .unwrap_or_else(|| Err("Cannot determine Kiro plugin directory".to_string())),
        _ => Ok(()),
    }
}

fn register_kiro_plugin(plugin_dir: &Path) -> Result<(), String> {
    let bundle = platform::bundle_dir().join("kiro");
    if !bundle.is_dir() {
        return Err("kiro bundle does not exist, please run assemble first".to_string());
    }

    // Deploy skills to ~/.kiro/skills/ — each skill in its own directory
    // Naming strategy: use command name directly; if a non-dev-flow managed skill with the same name exists, add dow- prefix
    let skills_src = bundle.join("skills");
    let mut conflicts: Vec<(String, String)> = Vec::new();
    if skills_src.is_dir() {
        fs::create_dir_all(plugin_dir)
            .map_err(|e| format!("Failed to create kiro skills directory: {}", e))?;
        if let Ok(entries) = fs::read_dir(&skills_src) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let skill_name = entry.file_name();
                let name_str = skill_name.to_string_lossy().to_string();
                let dst_skill = plugin_dir.join(&skill_name);
                let is_ours = dst_skill.join(".dev-flow-managed").exists();
                let final_dst = if dst_skill.exists() && !is_ours {
                    let prefixed = format!("dow-{}", name_str);
                    conflicts.push((name_str, prefixed.clone()));
                    plugin_dir.join(prefixed)
                } else {
                    dst_skill
                };
                if final_dst.exists() {
                    let _ = fs::remove_dir_all(&final_dst);
                }
                agent_registry::copy_dir_recursive(&entry.path(), &final_dst)
                    .map_err(|e| format!("Failed to deploy kiro skill {:?}: {}", skill_name, e))?;
            }
        }
    }
    if !conflicts.is_empty() {
        eprintln!("[dow] kiro skill naming conflicts:");
        for (orig, renamed) in &conflicts {
            eprintln!(
                "  {} → {} (skill with same name already exists)",
                orig, renamed
            );
        }
    }

    // Deploy agents to ~/.kiro/agents/<name>.json (flat files)
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let agents_src = bundle.join("agents");
    let agents_dst = PathBuf::from(&home).join(".kiro").join("agents");
    if agents_src.is_dir() {
        fs::create_dir_all(&agents_dst)
            .map_err(|e| format!("Failed to create kiro agents directory: {}", e))?;
        if let Ok(entries) = fs::read_dir(&agents_src) {
            for entry in entries.flatten() {
                let src_path = entry.path();
                if src_path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let file_name = entry.file_name();
                let dst_file = agents_dst.join(&file_name);
                fs::copy(&src_path, &dst_file)
                    .map_err(|e| format!("Failed to deploy kiro agent {:?}: {}", file_name, e))?;
            }
        }
    }

    eprintln!("[dow] Tip: Run /agent set-default dev-flow to set dev-flow as the default agent for kiro-cli");

    Ok(())
}

fn register_codex_plugin(plugin_dir: &Path) -> Result<(), String> {
    let codex_bin = resolve_codex_binary()?;
    eprintln!("[dow] Using Codex runtime: {}", codex_bin.display());

    cleanup_legacy_codex_paths()?;
    install_codex_marketplace_manifest(plugin_dir)?;
    let stage_root = install_codex_personal_marketplace(plugin_dir)?;
    install_codex_personal_skill(plugin_dir)?;
    set_codex_feature_enabled("plugin_hooks", true)?;
    set_codex_root_bool("suppress_unstable_features_warning", true)?;

    let add = std::process::Command::new(&codex_bin)
        .args([
            "plugin",
            "marketplace",
            "add",
            stage_root.to_str().unwrap_or(""),
        ])
        .output();

    match add {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Codex marketplace registration command failed: {}",
                stderr.trim()
            ));
        }
        Err(e) => return Err(format!("Codex runtime invocation failed: {}", e)),
    }

    // Remove first then add to ensure Codex reinstalls plugin to correct cache structure
    let _ = std::process::Command::new(&codex_bin)
        .args(["plugin", "remove", "dev-flow@dev-flow-local"])
        .output();

    let install = std::process::Command::new(&codex_bin)
        .args(["plugin", "add", "dev-flow@dev-flow-local"])
        .output();

    match install {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Codex plugin install failed: {}", stderr.trim()));
        }
        Err(e) => return Err(format!("Codex plugin install invocation failed: {}", e)),
    }

    remove_codex_config_section("[marketplaces.dev-flow]")?;
    set_codex_plugin_enabled("dev-flow@dev-flow", false)?;
    set_codex_plugin_enabled("dev-flow@dev-flow-local", true)?;
    trust_codex_plugin_hooks(plugin_dir)?;
    Ok(())
}

fn resolve_codex_binary() -> Result<PathBuf, String> {
    if let Ok(explicit) = env::var("CODEX_BIN") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            let path = PathBuf::from(explicit);
            if is_executable_file(&path) {
                return Ok(path);
            }
            return Err(format!(
                "CODEX_BIN does not point to an executable file: {}",
                path.display()
            ));
        }
    }

    // Prefer the runtime bundled with Codex/ChatGPT App. The app may be the
    // only installed Codex surface and does not have to expose `codex` in PATH.
    if let Some(path) = platform::codex_cli_candidates()
        .into_iter()
        .find(|path| is_executable_file(path))
    {
        return Ok(path);
    }

    // Fall back to a standalone CLI or a user-provided PATH entry.
    if let Some(path) = executable_on_path("codex") {
        return Ok(path);
    }

    Err(
        "Codex runtime not found. Set CODEX_BIN or install Codex/ChatGPT App with its bundled runtime."
            .to_string(),
    )
}

fn executable_on_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for directory in env::split_paths(&path_var) {
        let candidate = directory.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }

        #[cfg(windows)]
        {
            let candidate = directory.join(format!("{}.exe", name));
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn cleanup_legacy_codex_paths() -> Result<(), String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine user HOME directory".to_string())?;
    let home = Path::new(&home);
    let legacy_plugin = home
        .join(".codex")
        .join("plugins")
        .join("plugins")
        .join(CODEX_PLUGIN_NAME);
    if legacy_plugin.exists() {
        fs::remove_dir_all(&legacy_plugin)
            .map_err(|e| format!("Failed to clean up old Codex plugin directory: {}", e))?;
    }
    let legacy_cache = home
        .join(".codex")
        .join("plugins")
        .join("cache")
        .join(CODEX_MARKETPLACE_NAME);
    if legacy_cache.exists() {
        fs::remove_dir_all(&legacy_cache)
            .map_err(|e| format!("Failed to clean up old Codex plugin cache: {}", e))?;
    }
    Ok(())
}

fn install_codex_personal_skill(plugin_dir: &Path) -> Result<(), String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine user HOME directory".to_string())?;
    let source = plugin_dir.join("skills").join(CODEX_PLUGIN_NAME);
    let target = Path::new(&home)
        .join(".agents")
        .join("skills")
        .join(CODEX_PLUGIN_NAME);

    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to clean up Codex skill: {}", e))?;
    }

    // New Codex bundles expose command skills directly and do not include the
    // old aggregate skills/dev-flow skill. Keep that bundle layout valid while
    // still supporting older bundles that provide the aggregate skill.
    if !source.is_dir() {
        return Ok(());
    }

    copy_dir_recursive(&source, &target)
        .map_err(|e| format!("Failed to install Codex skill: {}", e))
}

fn install_codex_marketplace_manifest(plugin_dir: &Path) -> Result<(), String> {
    let source = plugin_dir
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if !source.exists() {
        return Err(format!(
            "Codex marketplace manifest missing from bundle: {}",
            source.display()
        ));
    }
    Ok(())
}

fn install_codex_personal_marketplace(plugin_dir: &Path) -> Result<std::path::PathBuf, String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine user HOME directory".to_string())?;
    let home = Path::new(&home);
    let marketplace_dir = platform::codex_personal_marketplace_dir()
        .ok_or_else(|| "Cannot determine Codex personal marketplace directory".to_string())?;
    let plugin_root = marketplace_dir
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| "Cannot determine Codex personal marketplace root directory".to_string())?;
    let source = plugin_dir
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if !source.exists() {
        return Err(format!(
            "Codex personal marketplace manifest missing from bundle: {}",
            source.display()
        ));
    }

    let target = marketplace_dir.join("marketplace.json");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create Codex personal marketplace directory: {}",
                e
            )
        })?;
    }
    fs::copy(&source, &target)
        .map_err(|e| format!("Failed to write Codex personal marketplace: {}", e))?;

    let plugin_target = platform::agent_plugin_dir("codex")
        .ok_or_else(|| "Cannot determine Codex plugin directory".to_string())?;
    let stage_root = home
        .join(".codex")
        .join(".tmp")
        .join("marketplaces")
        .join(CODEX_MARKETPLACE_NAME);
    if stage_root.exists() {
        fs::remove_dir_all(&stage_root)
            .map_err(|e| format!("Failed to clean up Codex marketplace stage: {}", e))?;
    }
    fs::create_dir_all(&stage_root)
        .map_err(|e| format!("Failed to create Codex marketplace stage: {}", e))?;
    let stage_marketplace = stage_root
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if let Some(parent) = stage_marketplace.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Codex marketplace stage directory: {}", e))?;
    }
    fs::copy(&source, &stage_marketplace)
        .map_err(|e| format!("Failed to write Codex marketplace stage: {}", e))?;
    let stage_plugin_target = stage_root.join("plugins").join(CODEX_PLUGIN_NAME);
    copy_dir_recursive(&plugin_target, &stage_plugin_target)
        .map_err(|e| format!("Failed to install Codex marketplace stage plugin: {}", e))?;
    let install_meta = stage_root.join(".codex-marketplace-install.json");
    let install_meta_body = format!(
        "{{\n  \"source_type\": \"local\",\n  \"source\": \"{}\",\n  \"ref_name\": null,\n  \"sparse_paths\": [],\n  \"revision\": null\n}}\n",
        plugin_root.display()
    );
    fs::write(&install_meta, install_meta_body)
        .map_err(|e| format!("Failed to write Codex marketplace install metadata: {}", e))?;

    Ok(stage_root)
}

fn trust_codex_plugin_hooks(plugin_dir: &Path) -> Result<(), String> {
    let hooks_path = plugin_dir.join("hooks").join("hooks.json");
    if !hooks_path.exists() {
        let alt = plugin_dir.join("hooks.json");
        if !alt.exists() {
            return Ok(());
        }
        return trust_codex_hooks_from_file(&alt);
    }
    trust_codex_hooks_from_file(&hooks_path)
}

fn trust_codex_hooks_from_file(hooks_path: &Path) -> Result<(), String> {
    let content =
        fs::read_to_string(hooks_path).map_err(|e| format!("Failed to read hooks.json: {}", e))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse hooks.json: {}", e))?;

    let hooks_obj = parsed
        .get("hooks")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "hooks.json missing hooks field".to_string())?;

    let event_map: &[(&str, &str)] = &[
        ("UserPromptSubmit", "user_prompt_submit"),
        ("PreToolUse", "pre_tool_use"),
        ("PostToolUse", "post_tool_use"),
        ("PreCompact", "pre_compact"),
        ("PostCompact", "post_compact"),
        ("SessionStart", "session_start"),
        ("SubagentStart", "subagent_start"),
        ("SubagentStop", "subagent_stop"),
        ("PermissionRequest", "permission_request"),
        ("Stop", "stop"),
    ];

    let config_path = codex_config_path()?;
    let mut config_content = if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?
    } else {
        String::new()
    };

    // Clean up old hooks.state sections
    while let Some(start) = config_content.find("\n[hooks.state.\"dev-flow@dev-flow-local:") {
        let section_end = config_content[start + 1..]
            .find("\n[")
            .map(|offset| start + 1 + offset)
            .unwrap_or(config_content.len());
        config_content.replace_range(start..section_end, "");
    }
    // Also handle cases at the beginning
    if config_content.starts_with("[hooks.state.\"dev-flow@dev-flow-local:") {
        let section_end = config_content[1..]
            .find("\n[")
            .map(|offset| 1 + offset)
            .unwrap_or(config_content.len());
        config_content.replace_range(0..section_end, "");
    }
    // Clean up empty [hooks.state] header
    config_content = config_content.replace("\n[hooks.state]\n\n", "\n");
    config_content = config_content.replace("\n[hooks.state]\n", "\n");

    let mut trust_sections = String::new();
    trust_sections.push_str("\n[hooks.state]\n");

    for (json_event, key_event) in event_map {
        let groups = match hooks_obj.get(*json_event).and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => continue,
        };

        for (group_idx, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(|v| v.as_str());
            let hooks_arr = match group.get("hooks").and_then(|v| v.as_array()) {
                Some(arr) => arr,
                None => continue,
            };

            for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                if hook.get("type").and_then(|v| v.as_str()) != Some("command") {
                    continue;
                }
                let command = match hook.get("command").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => continue,
                };
                let timeout = hook
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(600)
                    .max(1);
                let is_async = hook.get("async").and_then(|v| v.as_bool()).unwrap_or(false);

                let hash = compute_hook_hash(key_event, matcher, command, timeout, is_async);
                let key = format!(
                    "dev-flow@dev-flow-local:hooks/hooks.json:{}:{}:{}",
                    key_event, group_idx, hook_idx
                );

                trust_sections.push_str(&format!(
                    "\n[hooks.state.\"{}\"]\ntrusted_hash = \"{}\"\nenabled = true\n",
                    key, hash
                ));
            }
        }
    }

    if !config_content.ends_with('\n') && !config_content.is_empty() {
        config_content.push('\n');
    }
    config_content.push_str(&trust_sections);

    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

fn compute_hook_hash(
    event_name: &str,
    matcher: Option<&str>,
    command: &str,
    timeout: u64,
    is_async: bool,
) -> String {
    // Construct the same NormalizedHookIdentity structure as Codex
    let mut identity = serde_json::Map::new();
    identity.insert(
        "event_name".to_string(),
        serde_json::Value::String(event_name.to_string()),
    );

    let mut hook_obj = serde_json::Map::new();
    hook_obj.insert("async".to_string(), serde_json::Value::Bool(is_async));
    hook_obj.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    hook_obj.insert(
        "timeout".to_string(),
        serde_json::Value::Number(serde_json::Number::from(timeout)),
    );
    hook_obj.insert(
        "type".to_string(),
        serde_json::Value::String("command".to_string()),
    );

    if let Some(m) = matcher {
        identity.insert(
            "matcher".to_string(),
            serde_json::Value::String(m.to_string()),
        );
    }
    identity.insert(
        "hooks".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::Object(hook_obj)]),
    );

    let canonical = canonical_json(&serde_json::Value::Object(identity));
    let serialized = serde_json::to_vec(&canonical).unwrap_or_default();
    let hash = Sha256::digest(&serialized);
    format!("sha256:{:x}", hash)
}

fn canonical_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(val) = map.get(key) {
                    sorted.insert(key.clone(), canonical_json(val));
                }
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonical_json).collect())
        }
        other => other.clone(),
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
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

fn set_codex_plugin_enabled(plugin_key: &str, enabled: bool) -> Result<(), String> {
    let config_path = codex_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Codex config directory: {}", e))?;
    }

    let mut content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read Codex config: {}", e))?
    } else {
        String::new()
    };

    let header = format!("[plugins.\"{}\"]", plugin_key);
    if let Some(start) = content.find(&header) {
        let section_body_start = start + header.len();
        let section_end = content[section_body_start..]
            .find("\n[")
            .map(|offset| section_body_start + offset)
            .unwrap_or(content.len());
        let section = &content[section_body_start..section_end];

        let enabled_line = format!("enabled = {}", enabled);
        if let Some(enabled_pos) = section.find("\nenabled") {
            let line_start = section_body_start + enabled_pos + 1;
            let line_end = content[line_start..]
                .find('\n')
                .map(|offset| line_start + offset)
                .unwrap_or(content.len());
            content.replace_range(line_start..line_end, &enabled_line);
        } else {
            content.insert_str(section_body_start, &format!("\n{}", enabled_line));
        }

        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write Codex config: {}", e))?;
        return Ok(());
    }

    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&header);
    content.push_str(&format!("\nenabled = {}\n", enabled));

    fs::write(&config_path, content).map_err(|e| format!("Failed to write Codex config: {}", e))?;

    Ok(())
}

fn remove_codex_config_section(header: &str) -> Result<(), String> {
    let config_path = codex_config_path()?;
    if !config_path.exists() {
        return Ok(());
    }

    let mut content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Codex config: {}", e))?;

    if let Some(start) = content.find(header) {
        let section_end = content[start + header.len()..]
            .find("\n[")
            .map(|offset| start + header.len() + offset)
            .unwrap_or(content.len());
        let remove_start = if start > 0 && content.as_bytes()[start - 1] == b'\n' {
            start - 1
        } else {
            start
        };
        content.replace_range(remove_start..section_end, "");
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write Codex config: {}", e))?;
    }

    Ok(())
}

fn set_codex_feature_enabled(feature: &str, enabled: bool) -> Result<(), String> {
    let config_path = codex_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Codex config directory: {}", e))?;
    }

    let mut content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read Codex config: {}", e))?
    } else {
        String::new()
    };

    let header = "[features]";
    let feature_line = format!("{} = {}", feature, enabled);
    let feature_prefix = format!("{} =", feature);

    if let Some(start) = content.find(header) {
        let section_body_start = start + header.len();
        let section_end = content[section_body_start..]
            .find("\n[")
            .map(|offset| section_body_start + offset)
            .unwrap_or(content.len());
        let section = &content[section_body_start..section_end];

        if let Some(feature_pos) = section.find(&feature_prefix) {
            let line_start = section_body_start + feature_pos;
            let line_end = content[line_start..]
                .find('\n')
                .map(|offset| line_start + offset)
                .unwrap_or(content.len());
            content.replace_range(line_start..line_end, &feature_line);
        } else {
            content.insert_str(section_end, &format!("\n{}\n", feature_line));
        }
    } else {
        if !content.ends_with('\n') && !content.is_empty() {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(header);
        content.push('\n');
        content.push_str(&feature_line);
        content.push('\n');
    }

    fs::write(&config_path, content).map_err(|e| format!("Failed to write Codex config: {}", e))?;
    Ok(())
}

fn set_codex_root_bool(key: &str, enabled: bool) -> Result<(), String> {
    let config_path = codex_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Codex config directory: {}", e))?;
    }

    let mut content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read Codex config: {}", e))?
    } else {
        String::new()
    };

    let key_prefix = format!("{} =", key);
    let key_line = format!("{} = {}", key, enabled);
    let first_section_start = if content.starts_with('[') {
        0
    } else {
        content
            .find("\n[")
            .map(|offset| offset + 1)
            .unwrap_or(content.len())
    };
    let root_section = &content[..first_section_start];

    if let Some(key_pos) = root_section.find(&key_prefix) {
        let line_end = content[key_pos..]
            .find('\n')
            .map(|offset| key_pos + offset)
            .unwrap_or(content.len());
        content.replace_range(key_pos..line_end, &key_line);
    } else {
        let prefix = if first_section_start == content.len()
            && !content.is_empty()
            && !content.ends_with('\n')
        {
            "\n"
        } else {
            ""
        };
        content.insert_str(first_section_start, &format!("{}{}\n", prefix, key_line));
    }

    fs::write(&config_path, content).map_err(|e| format!("Failed to write Codex config: {}", e))?;
    Ok(())
}

fn codex_config_path() -> Result<std::path::PathBuf, String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine user HOME directory".to_string())?;
    Ok(Path::new(&home).join(".codex").join("config.toml"))
}

fn agent_display_name(agent: &str) -> &str {
    agent_registry::SUPPORTED_AGENTS
        .iter()
        .find(|a| a.name == agent)
        .map(|a| a.display_name)
        .unwrap_or(agent)
}
