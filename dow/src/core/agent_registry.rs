// dow/src/core/agent_registry.rs
// Agent 插件目录发现与文件部署

use std::fs;
use std::path::Path;

use super::platform;

pub struct AgentInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}

pub const SUPPORTED_AGENTS: &[AgentInfo] = &[
    AgentInfo { name: "claude", display_name: "Claude Code" },
    AgentInfo { name: "codex", display_name: "Codex" },
];

pub fn deploy_plugin(agent: &str, bundle_dir: &Path) -> Result<(), String> {
    let source = bundle_dir.join(agent);
    if !source.exists() {
        return Err(format!("bundle 中未找到 {} 插件资源: {}", agent, source.display()));
    }

    let target = platform::agent_plugin_dir(agent)
        .ok_or_else(|| format!("不支持的 agent: {}", agent))?;

    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("清理旧插件目录失败: {}", e))?;
    }

    copy_dir_recursive(&source, &target)?;

    Ok(())
}

pub fn inject_global_instructions(agent: &str) -> Result<bool, String> {
    let instructions_path = platform::agent_global_instructions(agent)
        .ok_or_else(|| format!("不支持的 agent: {}", agent))?;

    let marker = "<!-- dev-flow -->";
    let content = if instructions_path.exists() {
        fs::read_to_string(&instructions_path)
            .map_err(|e| format!("读取全局指令文件失败: {}", e))?
    } else {
        String::new()
    };

    if content.contains(marker) {
        return Ok(false);
    }

    let injection = format!(
        "\n\n{}\n## Dev Flow\n\n\
         **MUST use dev-flow to manage development workflow.** Available commands:\n\n\
         | Command | Purpose |\n\
         |---|---|\n\
         | `/init` | Initialize project |\n\
         | `/brainstorm` | Collaborative requirement exploration |\n\
         | `/prd` | Start PRD phase |\n\
         | `/spec` | Start SPEC phase |\n\
         | `/task` | Start TASK phase |\n\
         | `/issue` | Create an issue |\n\
         | `/devtest` | Routine dev testing |\n\
         | `/fix` | Auto-fix open issues |\n\
         | `/test` | Full test phase |\n\
         | `/status` | Report status |\n\
         | `/check` | Check doc sync |\n\
         | `/iterate` | Iterate delivery |\n\
         | `/mode` | Select dev mode |\n",
        marker
    );

    if let Some(parent) = instructions_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let new_content = format!("{}{}", content, injection);
    fs::write(&instructions_path, new_content)
        .map_err(|e| format!("写入全局指令文件失败: {}", e))?;

    Ok(true)
}

pub fn verify_plugin_integrity(agent: &str) -> Result<Vec<String>, String> {
    let target = platform::agent_plugin_dir(agent)
        .ok_or_else(|| format!("不支持的 agent: {}", agent))?;

    let mut issues = Vec::new();

    if !target.exists() {
        issues.push(format!("插件目录不存在: {}", target.display()));
        return Ok(issues);
    }

    let required_dirs = ["skills", "commands", "agents"];
    for dir in &required_dirs {
        if !target.join(dir).exists() {
            issues.push(format!("缺少目录: {}/{}", target.display(), dir));
        }
    }

    match agent {
        "claude" => {
            if !target.join("hooks/hooks.json").exists() && !target.join("hooks").join("hooks.json").exists() {
                issues.push("缺少 hooks/hooks.json".to_string());
            }
            if !target.join(".claude-plugin/plugin.json").exists() {
                issues.push("缺少 .claude-plugin/plugin.json".to_string());
            }
        }
        "codex" => {
            if !target.join("hooks.json").exists() {
                issues.push("缺少 hooks.json".to_string());
            }
            if !target.join(".codex-plugin/plugin.json").exists() {
                issues.push("缺少 .codex-plugin/plugin.json".to_string());
            }
        }
        _ => {}
    }

    Ok(issues)
}

pub fn is_agent_available(agent: &str) -> bool {
    match agent {
        "claude" => which_command("claude"),
        "codex" => which_command("codex"),
        _ => false,
    }
}

fn which_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("创建目录 {} 失败: {}", dst.display(), e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("读取目录 {} 失败: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("复制 {} → {} 失败: {}", src_path.display(), dst_path.display(), e))?;
        }
    }
    Ok(())
}
