// dow/src/commands/
// ├── status.rs  -- dow status 子命令（读写 STATUS.yaml + mode 联动）
//
// Related Docs:
// - [STATUS 规范](../../../references/.dev-doc/STATUS.md)
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::cli::StatusArgs;
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

// mode 对应的阶段流程链
fn phase_chain(mode: &str) -> Vec<&'static str> {
    match mode {
        "full" => vec!["PRD", "SPEC", "TASK", "DEV", "TEST", "DONE"],
        "quick" => vec!["SPEC", "TASK", "DEV", "TEST", "DONE"],
        "fast" => vec!["TASK", "DEV", "TEST", "DONE"],
        "mvp" => vec!["SPEC", "TASK", "DEV", "DONE"],
        _ => vec!["DEV"],
    }
}

// mode 对应的起始阶段
fn mode_start_phase(mode: &str) -> &'static str {
    match mode {
        "full" => "PRD",
        "quick" | "mvp" => "SPEC",
        "fast" => "TASK",
        _ => "DEV",
    }
}

// 校验阶段跳转是否合法
fn validate_phase_transition(current: &str, target: &str, mode: &str) -> Result<(), String> {
    // TEST → DEV 始终允许（回退修 bug）
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
                    "非法跳转：{} → {}（{} 模式下只允许前进一步：{} → {}）",
                    current,
                    target,
                    mode,
                    current,
                    chain.get(ci + 1).unwrap_or(&"DONE")
                ))
            }
        }
        (None, Some(_)) => Ok(()), // 当前阶段不在链中，允许跳转
        (_, None) => Err(format!("无效阶段：{}（合法值：{}）", target, chain.join("/"))),
    }
}

pub fn run(args: StatusArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new(
            format!("STATUS.yaml 不存在：{}", status_file.display()),
            1,
        ));
    }

    // 写操作
    let is_write = args.phase.is_some()
        || args.mode.is_some()
        || args.exec_mode.is_some()
        || args.name.is_some()
        || args.goals_minor.is_some()
        || args.goals_major.is_some();

    if is_write {
        return handle_write(&status_file, &args);
    }

    // 读操作
    handle_read(&status_file, &doc_root_path, args.field, human)
}

fn handle_write(status_file: &PathBuf, args: &StatusArgs) -> Result<i32, DowError> {
    // 设置 phase（带合法性校验）
    if let Some(ref target_phase) = args.phase {
        let target = target_phase.to_uppercase();
        let current = yaml::get(status_file, "phase")
            .map_err(|e| DowError::new(e.to_string(), 1))?
            .unwrap_or_default();
        let mode = yaml::get(status_file, "mode")
            .map_err(|e| DowError::new(e.to_string(), 1))?
            .unwrap_or_else(|| "quick".to_string());

        // 提取 effective mode（去掉 audit/ 前缀）
        let effective_mode = if mode.starts_with("audit/") {
            &mode[6..]
        } else {
            &mode
        };

        validate_phase_transition(&current, &target, effective_mode)
            .map_err(|e| DowError::new(e, 1))?;

        // 进入 DEV 的前提：存在未关闭的 task 或 issue，且文档合法
        if target == "DEV" {
            let doc_root_path = status_file.parent().unwrap();
            let has_tasks = has_open_tasks(doc_root_path);
            let has_issues = has_open_issues(doc_root_path);
            if !has_tasks && !has_issues {
                return Err(DowError::new(
                    "无法进入 DEV：不存在未关闭的 task 或 issue。请先用 `dow doc task` 或 `dow doc issue` 创建。",
                    1,
                ));
            }
            // 文档合法性校验
            let validation_errors = doc_validator::validate_all(doc_root_path);
            if !validation_errors.is_empty() {
                let msg = format!(
                    "无法进入 DEV：.dev-doc 文件存在格式错误。\n{}",
                    doc_validator::format_errors_human(&validation_errors)
                );
                return Err(DowError::new(msg, 1));
            }
        }

        yaml::set(status_file, "phase", &target)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 设置 mode（拒绝 audit + 联动 phase）
    if let Some(ref new_mode) = args.mode {
        if new_mode.starts_with("audit") {
            return Err(DowError::new(
                "audit 模式为自动触发，不支持手动设置",
                1,
            ));
        }
        let valid_modes = ["full", "quick", "fast", "mvp"];
        if !valid_modes.contains(&new_mode.as_str()) {
            return Err(DowError::new(
                format!("无效模式：{}（可选：full/quick/fast/mvp）", new_mode),
                1,
            ));
        }

        yaml::set(status_file, "mode", new_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;

        // 联动 phase 起点
        let start_phase = mode_start_phase(new_mode);
        yaml::set(status_file, "phase", start_phase)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 设置 exec_mode
    if let Some(ref exec_mode) = args.exec_mode {
        let valid = ["step", "continuous"];
        if !valid.contains(&exec_mode.as_str()) {
            return Err(DowError::new(
                format!("无效 exec_mode：{}（可选：step/continuous）", exec_mode),
                1,
            ));
        }
        yaml::set(status_file, "exec_mode", exec_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 设置 name（不允许为空）
    if let Some(ref name) = args.name {
        if name.trim().is_empty() {
            return Err(DowError::new("name 不能为空", 1));
        }
        yaml::set(status_file, "name", name)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 设置版本目标
    if let Some(ref goal) = args.goals_minor {
        yaml::set(status_file, "goals_minor", goal)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }
    if let Some(ref goal) = args.goals_major {
        yaml::set(status_file, "goals_major", goal)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 自动更新 updated 时间戳
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

    // 只取某字段
    if let Some(ref key) = field {
        let value = map.get(key).cloned().unwrap_or_default();
        println!("{}", value);
        return Ok(0);
    }

    // 读取 VERSION 文件
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
    println!("[dev-flow] 项目状态报告");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("项目名称：{}", status.name);
    println!("文档根：{}", status.doc_root);
    println!("当前阶段：{}", status.phase);
    println!("开发模式：{}", status.mode);
    println!("执行模式：{}", status.exec_mode);
    println!("当前版本：({})v{} ({})", branch, status.version, status.version_tag);
    if let Some(ref g) = status.goals_minor {
        println!("目标(minor)：{}", g);
    }
    if let Some(ref g) = status.goals_major {
        println!("目标(major)：{}", g);
    }
    println!("更新时间：{}", status.updated);
    println!("启动时间：{}", status.started);
}

/// 检查是否存在未完成的 task
fn has_open_tasks(doc_root: &std::path::Path) -> bool {
    let task_dir = doc_root.join("task");
    crate::core::task_store::has_active_work(&task_dir)
}

/// 检查是否存在未关闭的 issue
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
