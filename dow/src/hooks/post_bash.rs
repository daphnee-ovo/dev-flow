// dow/src/hooks/
// ├── post_bash.rs  -- detect branch switch after Bash execution
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)

use crate::core::doc_root;
use crate::error::DowError;
use serde_json;
use std::io::Read as IoRead;
use std::path::Path;

pub fn run(command: Option<String>, codex_hook: bool, _kiro_hook: bool) -> Result<i32, DowError> {
    if !Path::new(crate::core::DOC_DIR).is_dir() {
        return Ok(0);
    }

    let cmd = command
        .or_else(|| {
            // read Claude Code hook JSON from stdin
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok()?;
            serde_json::from_str::<serde_json::Value>(&buf)
                .ok()
                .and_then(|j| {
                    j.get("tool_input")
                        .and_then(|ti| ti.get("command"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
        })
        .unwrap_or_default();

    if cmd.is_empty() {
        return Ok(0);
    }

    // Detect if branch switch command was executed
    if !is_branch_switch_command(&cmd) {
        return Ok(0);
    }

    // Get current branch and report
    let branch = doc_root::current_branch().unwrap_or_else(|| "unknown".to_string());
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    let mut messages = vec![format!(
        "[dev-flow] Branch switch detected → current branch: `{}`, doc_root: {}",
        branch,
        doc_root_path.display()
    )];

    // Check if new branch has STATUS.yaml (resolve creates it automatically, but show hint here)
    if doc_root_path.join("STATUS.yaml").exists() {
        let phase = crate::core::yaml::get(&doc_root_path.join("STATUS.yaml"), "phase")
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());
        messages.push(format!("  Phase: {}, doc directory ready.", phase));
    } else {
        messages.push("  ⚠ New branch not yet initialized with .dev-doc, will be created automatically.".to_string());
    }

    emit_messages(codex_hook, &messages)?;
    Ok(0)
}

fn emit_messages(codex_hook: bool, messages: &[String]) -> Result<(), DowError> {
    if messages.is_empty() {
        return Ok(());
    }
    if codex_hook {
        return Ok(());
    } else {
        for message in messages {
            println!("{}", message);
        }
    }
    Ok(())
}

fn is_branch_switch_command(cmd: &str) -> bool {
    let patterns = [
        "git checkout",
        "git switch",
        "git checkout -b",
        "git switch -c",
    ];
    patterns.iter().any(|p| cmd.contains(p))
}
