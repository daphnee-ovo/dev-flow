// dow/src/commands/
// ├── status.rs  -- dow status subcommand (read/write STATUS.yaml + mode coordination)
//
// Related Docs:
// - [STATUS specification](../../../references/.dev-doc/STATUS.md)
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::cli::{StatusArgs, StatusCommands, StatusSetArgs};
use crate::error::DowError;
use crate::core::{doc_root, doc_validator, yaml};
use crate::output;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct StatusOutput {
    version: String,
    version_tag: String,
    name: String,
    phase: String,
    mode: String,
    exec_mode: String,
    doc_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    goals_minor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goals_major: Option<String>,
    updated: String,
    started: String,
}

// Phase chain corresponding to mode
fn phase_chain(mode: &str) -> Vec<&'static str> {
    match mode {
        "full" => vec!["PRD", "SPEC", "TASK", "DEV", "TEST", "DONE"],
        "quick" => vec!["SPEC", "TASK", "DEV", "TEST", "DONE"],
        "fast" => vec!["TASK", "DEV", "TEST", "DONE"],
        "mvp" => vec!["SPEC", "TASK", "DEV", "DONE"],
        _ => vec!["DEV"],
    }
}

// Validate whether phase transition is legal
fn validate_phase_transition(current: &str, target: &str, mode: &str) -> Result<(), String> {
    // TEST → DEV always allowed (rollback to fix bugs)
    if current == "TEST" && target == "DEV" {
        return Ok(());
    }

    let chain = phase_chain(mode);
    let current_idx = chain.iter().position(|&p| p == current);
    let target_idx = chain.iter().position(|&p| p == target);

    match (current_idx, target_idx) {
        (Some(ci), Some(ti)) => {
            if ti == ci + 1 {
                Ok(())
            } else {
                Err(format!(
                    "Illegal transition: {} → {} (in {} mode, only one step forward is allowed: {} → {})",
                    current,
                    target,
                    mode,
                    current,
                    chain.get(ci + 1).unwrap_or(&"DONE")
                ))
            }
        }
        (None, Some(_)) => Ok(()), // Current phase not in chain, allow transition
        (_, None) => Err(format!("Invalid phase: {} (valid values: {})", target, chain.join("/"))),
    }
}

pub fn run(args: StatusArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new(
            format!("STATUS.yaml does not exist: {}", status_file.display()),
            1,
        ));
    }

    // Dispatch: `dow status set ...` vs `dow status [--field]`
    match args.command {
        Some(StatusCommands::Set(set_args)) => handle_write(&status_file, &set_args),
        None => handle_read(&status_file, &doc_root_path, args.field, human),
    }
}

fn handle_write(status_file: &PathBuf, args: &StatusSetArgs) -> Result<i32, DowError> {
    // Set phase (with validity check)
    if let Some(ref target_phase) = args.phase {
        let target = target_phase.to_uppercase();
        let current = yaml::get(status_file, "phase")
            .map_err(|e| DowError::new(e.to_string(), 1))?
            .unwrap_or_default();
        let mode = yaml::get(status_file, "mode")
            .map_err(|e| DowError::new(e.to_string(), 1))?
            .unwrap_or_else(|| "quick".to_string());

        // Extract effective mode (remove audit/ prefix)
        let effective_mode = if mode.starts_with("audit/") {
            &mode[6..]
        } else {
            &mode
        };

        validate_phase_transition(&current, &target, effective_mode)
            .map_err(|e| DowError::new(e, 1))?;

        // Prerequisite for entering DEV: existence of open tasks or issues, and valid documentation
        if target == "DEV" {
            let doc_root_path = status_file.parent().unwrap();
            let has_tasks = has_open_tasks(doc_root_path);
            let has_issues = has_open_issues(doc_root_path);
            if !has_tasks && !has_issues {
                return Err(DowError::new(
                    "Cannot enter DEV: no open tasks or issues exist. Please create them first using `dow task create` or `dow issue create`.",
                    1,
                ));
            }
            // Documentation validity check
            let validation_errors = doc_validator::validate_all(doc_root_path);
            if !validation_errors.is_empty() {
                let msg = format!(
                    "Cannot enter DEV: .dev-doc files have format errors.\n{}",
                    doc_validator::format_errors_human(&validation_errors)
                );
                return Err(DowError::new(msg, 1));
            }
        }

        yaml::set(status_file, "phase", &target)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Set mode (reject audit + coordinate with phase)
    if let Some(ref new_mode) = args.mode {
        if new_mode.starts_with("audit") {
            return Err(DowError::new(
                "audit mode is auto-triggered and does not support manual setting",
                1,
            ));
        }
        let valid_modes = ["full", "quick", "fast", "mvp"];
        if !valid_modes.contains(&new_mode.as_str()) {
            return Err(DowError::new(
                format!("Invalid mode: {} (options: full/quick/fast/mvp)", new_mode),
                1,
            ));
        }

        yaml::set(status_file, "mode", new_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;

        // Mode switching does not coordinate with phase — phase is managed by explicit --phase or /iterate
        // Only validate whether current phase is legal under new mode, warn but don't modify if illegal
        let current_phase = yaml::get(status_file, "phase")
            .ok()
            .flatten()
            .unwrap_or_default();
        let chain = phase_chain(new_mode);
        if !chain.contains(&current_phase.as_str()) {
            eprintln!(
                "[dow] Warning: current phase {} is not in the workflow chain of {} mode ({}), manual adjustment recommended",
                current_phase, new_mode, chain.join(" → ")
            );
        }
    }

    // Set exec_mode
    if let Some(ref exec_mode) = args.exec_mode {
        let valid = ["step", "continuous"];
        if !valid.contains(&exec_mode.as_str()) {
            return Err(DowError::new(
                format!("Invalid exec_mode: {} (options: step/continuous)", exec_mode),
                1,
            ));
        }
        yaml::set(status_file, "exec_mode", exec_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Set name (must not be empty)
    if let Some(ref name) = args.name {
        if name.trim().is_empty() {
            return Err(DowError::new("name cannot be empty", 1));
        }
        yaml::set(status_file, "name", name)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Set version goals
    if let Some(ref goal) = args.goals_minor {
        yaml::set(status_file, "goals_minor", goal)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }
    if let Some(ref goal) = args.goals_major {
        yaml::set(status_file, "goals_major", goal)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Auto-update updated timestamp
    yaml::touch_updated(status_file)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    Ok(0)
}

fn handle_read(
    status_file: &PathBuf,
    doc_root_path: &PathBuf,
    field: Option<String>,
    human: bool,
) -> Result<i32, DowError> {
    let map = yaml::read(status_file).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Get only a specific field
    if let Some(ref key) = field {
        // Special handling for array fields
        if key == "docs" {
            let items = yaml::get_list(status_file, key)
                .map_err(|e| DowError::new(e.to_string(), 1))?;
            if human {
                for item in &items {
                    println!("{}", item);
                }
            } else {
                output::print_json(&items);
            }
            return Ok(0);
        }
        let value = map.get(key).cloned().unwrap_or_default();
        println!("{}", value);
        return Ok(0);
    }

    // Read VERSION file
    let (version, version_tag) = read_version_info();

    let status = StatusOutput {
        version,
        version_tag,
        name: map.get("name").cloned().unwrap_or_default(),
        phase: map.get("phase").cloned().unwrap_or_default(),
        mode: map.get("mode").cloned().unwrap_or_default(),
        exec_mode: map.get("exec_mode").cloned().unwrap_or_else(|| "step".to_string()),
        doc_root: doc_root_path.to_string_lossy().to_string(),
        goals_minor: map.get("goals_minor").cloned().filter(|s| !s.is_empty()),
        goals_major: map.get("goals_major").cloned().filter(|s| !s.is_empty()),
        updated: map.get("updated").cloned().unwrap_or_default(),
        started: map.get("started").cloned().unwrap_or_default(),
    };

    if human {
        print_human(&status);
    } else {
        output::print_json(&status);
    }

    Ok(0)
}

fn read_version_info() -> (String, String) {
    use crate::core::version;

    let version = version::read_current().unwrap_or_else(|_| "0.0.0".to_string());

    let tag_status = std::process::Command::new("git")
        .args(["tag", "-l", &format!("v{}", version)])
        .output()
        .map(|o| {
            let output = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if output.is_empty() {
                "no-tag".to_string()
            } else {
                "tagged".to_string()
            }
        })
        .unwrap_or_else(|_| "no-tag".to_string());

    (version, tag_status)
}

fn print_human(status: &StatusOutput) {
    let branch = crate::core::doc_root::current_branch()
        .unwrap_or_else(|| "main".to_string());
    println!("[dev-flow] Project Status Report");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("Project Name: {}", status.name);
    println!("Doc Root: {}", status.doc_root);
    println!("Current Phase: {}", status.phase);
    println!("Development Mode: {}", status.mode);
    println!("Execution Mode: {}", status.exec_mode);
    println!("Current Version: ({})v{} ({})", branch, status.version, status.version_tag);
    if let Some(ref g) = status.goals_minor {
        println!("Goal (minor): {}", g);
    }
    if let Some(ref g) = status.goals_major {
        println!("Goal (major): {}", g);
    }
    println!("Updated: {}", status.updated);
    println!("Started: {}", status.started);
}

/// Check if there are any open tasks
fn has_open_tasks(doc_root: &std::path::Path) -> bool {
    let task_dir = doc_root.join("task");
    crate::core::task_store::has_active_work(&task_dir)
}

/// Check if there are any open issues
fn has_open_issues(doc_root: &std::path::Path) -> bool {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return false;
    }
    if let Ok(entries) = std::fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("issue_") && name.ends_with(".md") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if content.lines().any(|l| l.starts_with("- [ ]")) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
