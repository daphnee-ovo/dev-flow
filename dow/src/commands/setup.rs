// dow/src/commands/setup.rs
// dow setup 子命令 — TUI 交互 + agent 注册

use dialoguer::MultiSelect;

use crate::core::{agent_registry, config::DowConfig, platform};
use crate::error::DowError;

pub fn run(agent: Option<String>, _human: bool) -> Result<i32, DowError> {
    let agents = resolve_agents(agent)?;

    let bundle = platform::bundle_dir();
    if !bundle.exists() {
        return Err(DowError::new(
            "插件 bundle 不存在。请先运行安装脚本或检查 ~/.local/share/dow/bundle/ 目录。",
            1,
        ));
    }

    let mut config = DowConfig::load();

    for agent_name in &agents {
        eprint!("[dow] 注册到 {}...", agent_display_name(agent_name));

        agent_registry::deploy_plugin(agent_name, &bundle)
            .map_err(|e| DowError::new(&format!("部署 {} 插件失败: {}", agent_name, e), 1))?;

        match agent_registry::inject_global_instructions(agent_name) {
            Ok(true) => eprintln!(" 已注入全局指令"),
            Ok(false) => eprintln!(" 全局指令已存在，跳过"),
            Err(e) => eprintln!(" 全局指令注入失败: {}", e),
        }

        register_with_agent(agent_name);

        config.add_agent(agent_name);
        eprintln!("[dow] ✓ {} 注册完成", agent_display_name(agent_name));
    }

    config.save().map_err(|e| DowError::new(&e, 1))?;

    eprintln!("\n[dow] 完成！在项目中运行 /init 开始使用 dev-flow。");
    Ok(0)
}

fn resolve_agents(agent_arg: Option<String>) -> Result<Vec<String>, DowError> {
    match agent_arg.as_deref() {
        Some("all") => Ok(agent_registry::SUPPORTED_AGENTS
            .iter()
            .map(|a| a.name.to_string())
            .collect()),
        Some(name) => {
            if agent_registry::SUPPORTED_AGENTS.iter().any(|a| a.name == name) {
                Ok(vec![name.to_string()])
            } else {
                Err(DowError::new(
                    &format!("不支持的 agent: {}（可选: claude, codex, all）", name),
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
        .with_prompt("选择要注册的 agent（空格选择，Enter 确认）")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .map_err(|e| DowError::new(&format!("交互选择失败: {}", e), 1))?;

    if selections.is_empty() {
        return Err(DowError::new("未选择任何 agent", 1));
    }

    let all_index = items.len() - 1;
    if selections.contains(&all_index) {
        return Ok(agent_registry::SUPPORTED_AGENTS
            .iter()
            .map(|a| a.name.to_string())
            .collect());
    }

    Ok(selections.iter()
        .map(|&i| agent_registry::SUPPORTED_AGENTS[i].name.to_string())
        .collect())
}

fn register_with_agent(agent: &str) {
    match agent {
        "claude" => {
            if let Some(plugin_dir) = platform::agent_plugin_dir("claude") {
                let path_str = plugin_dir.to_str().unwrap_or("");

                // 注册/更新 marketplace 源
                let _ = std::process::Command::new("claude")
                    .args(["plugin", "marketplace", "add", path_str])
                    .output();

                // 尝试 update；如果未安装则 install
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
        }
        _ => {}
    }
}

fn agent_display_name(agent: &str) -> &str {
    agent_registry::SUPPORTED_AGENTS
        .iter()
        .find(|a| a.name == agent)
        .map(|a| a.display_name)
        .unwrap_or(agent)
}
