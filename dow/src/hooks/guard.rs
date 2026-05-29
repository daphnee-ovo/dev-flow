// dow/src/hooks/
// ├── guard.rs  -- dow hooks guard（文件写入守护）
//    合并 block-system-tmp.sh + block-non-dev-edit.sh

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::path::Path;

pub fn run(file: String) -> Result<i32, DowError> {
    // 收集需要检查的文件路径列表
    let targets = resolve_targets(&file);

    if targets.is_empty() {
        return Ok(0);
    }

    for target in &targets {
        // 路径穿越检测：resolve 后判断是否在项目目录内
        let resolved = resolve_absolute(target);
        if !is_within_project(&resolved) {
            println!("[dev-flow] BLOCKED: 路径穿越，最终目标在项目外：{}", target);
            println!("→ resolved: {}", resolved);
            return Ok(1);
        }

        // block-system-tmp: 阻止写入系统临时目录
        if is_system_tmp(&resolved) {
            println!("[dev-flow] BLOCKED: 禁止写入系统临时目录：{}", target);
            println!("→ 请使用项目内的 tmp/ 或 temp/ 目录。");
            return Ok(1);
        }

        // block-version-direct-write: 禁止 agent 直接修改 VERSION 文件
        if is_version_file(target) {
            println!("[dev-flow] BLOCKED: 禁止直接修改 VERSION 文件");
            println!("→ 请使用 `dow version --set X.Y.Z` 或 `dow version --bump minor` 管理版本。");
            return Ok(1);
        }

        // block-status-direct-write: 禁止 agent 直接创建/修改 STATUS.yaml
        if is_status_file(target) {
            println!("[dev-flow] BLOCKED: 禁止直接创建或修改 STATUS.yaml");
            println!("→ 请使用 `dow status --phase/--mode/--name` 或 `dow init` 管理状态。");
            return Ok(1);
        }

        // block-devdoc-direct-create: 禁止 agent 手动创建 dev-doc 文档文件
        if let Some(msg) = check_devdoc_direct_create(target) {
            println!("{}", msg);
            return Ok(1);
        }

        // block-cross-branch: 拦截写入其他分支的 dev-doc 目录
        if let Some(reason) = check_cross_branch_write(target) {
            println!("{}", reason);
            return Ok(1);
        }

        // block-non-dev-edit: 非 DEV 阶段阻止代码编辑
        if is_code_file(target) {
            if let Some(reason) = check_non_dev_block(target) {
                println!("{}", reason);
                return Ok(1);
            }
        }
    }

    Ok(0)
}

/// 从参数和环境变量中提取写入目标
/// Write/Edit: file 参数直接是路径
/// Bash: file 为空，从 TOOL_INPUT 环境变量解析命令中的写入目标
fn resolve_targets(file: &str) -> Vec<String> {
    if !file.is_empty() {
        return vec![file.to_string()];
    }

    // Bash 场景：从 TOOL_INPUT 提取命令
    let tool_input = std::env::var("TOOL_INPUT").unwrap_or_default();
    if tool_input.is_empty() {
        return vec![];
    }

    // 解析 JSON 获取 command 字段
    let command = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&tool_input) {
        json.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    } else {
        return vec![];
    };

    extract_write_targets_from_command(&command)
}

/// 从 Bash 命令中提取可能的写入目标路径
fn extract_write_targets_from_command(cmd: &str) -> Vec<String> {
    let mut targets = Vec::new();

    // 匹配重定向写入：> file、>> file
    for part in cmd.split('>') {
        if part.is_empty() {
            continue;
        }
        // 取重定向后的第一个 token 作为目标文件
        let after = part.trim_start_matches('>').trim();
        if let Some(path) = after.split_whitespace().next() {
            let clean = path.trim_matches('"').trim_matches('\'');
            if looks_like_path(clean) {
                targets.push(clean.to_string());
            }
        }
    }

    // 匹配 tee 写入：| tee file、| tee -a file
    for segment in cmd.split("tee") {
        if segment.ends_with("| ") || segment.ends_with("|") || segment.ends_with("| \\\n") {
            continue;
        }
        // tee 后面的参数
        let after = segment.trim_start();
        let skip_flags: &[&str] = &["-a", "--append"];
        let mut tokens = after.split_whitespace();
        while let Some(token) = tokens.next() {
            if skip_flags.contains(&token) {
                continue;
            }
            let clean = token.trim_matches('"').trim_matches('\'');
            if looks_like_path(clean) {
                targets.push(clean.to_string());
                break;
            }
        }
    }

    // 匹配 cp/mv 目标：cp src dest、mv src dest
    for prefix in &["cp ", "mv "] {
        if let Some(pos) = cmd.find(prefix) {
            let args_str = &cmd[pos + prefix.len()..];
            let args_end = args_str.find(';')
                .or_else(|| args_str.find("&&"))
                .or_else(|| args_str.find('|'))
                .unwrap_or(args_str.len());
            let tokens: Vec<&str> = args_str[..args_end]
                .split_whitespace()
                .filter(|t| !t.starts_with('-'))
                .collect();
            // 最后一个非 flag 参数是目标
            if let Some(dest) = tokens.last() {
                let clean = dest.trim_matches('"').trim_matches('\'');
                if looks_like_path(clean) {
                    targets.push(clean.to_string());
                }
            }
        }
    }

    targets
}

fn looks_like_path(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') {
        return false;
    }
    s.contains('/') || s.contains('.') || s.starts_with("dev-doc")
}

fn resolve_absolute(file: &str) -> String {
    let path = Path::new(file);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut components: Vec<std::path::Component> = Vec::new();
    for comp in abs.components() {
        match comp {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    let resolved: std::path::PathBuf = components.iter().collect();
    resolved.to_string_lossy().to_string()
}

fn is_within_project(resolved_path: &str) -> bool {
    let project_root = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    resolved_path.starts_with(&project_root)
}

fn is_system_tmp(resolved: &str) -> bool {
    resolved.starts_with("/tmp/")
        || resolved.starts_with("/var/tmp/")
        || resolved.starts_with("/dev/shm/")
        || resolved.contains("/System/")
}

fn is_code_file(file: &str) -> bool {
    let code_exts = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".rs", ".go", ".java",
        ".rb", ".php", ".vue", ".svelte", ".sh",
    ];
    code_exts.iter().any(|ext| file.ends_with(ext))
}

fn check_cross_branch_write(file: &str) -> Option<String> {
    // 只检查 dev-doc/ 内的写入
    let normalized = file.replace('\\', "/");
    if !normalized.starts_with("dev-doc/") {
        return None;
    }

    // 提取写入目标的分支目录名（dev-doc/<branch>/...）
    let rest = &normalized["dev-doc/".len()..];
    let target_branch = rest.split('/').next().unwrap_or("");
    if target_branch.is_empty() {
        return None;
    }

    // 如果目标是文件（如 dev-doc/CHANGELOG.md）而非分支子目录，跳过
    if !Path::new(&format!("dev-doc/{}", target_branch)).is_dir()
        && !normalized.contains(&format!("{}/", target_branch))
    {
        return None;
    }

    // 获取当前分支
    let current = doc_root::current_branch();
    if let Some(ref branch) = current {
        if target_branch != branch.as_str() {
            return Some(format!(
                "[dev-flow] BLOCKED: 当前分支为 `{}`，禁止写入其他分支的文档目录：{}\n→ 请确认你已切换到正确的分支，或使用 `git checkout {}` 切换。",
                branch, file, target_branch
            ));
        }
    }

    None
}

fn check_non_dev_block(file: &str) -> Option<String> {
    // dev-doc 内的文件始终允许
    if file.starts_with("dev-doc/") || file.starts_with("dev-doc\\") {
        return None;
    }
    // tests/ 始终允许
    if file.starts_with("tests/") || file.starts_with("tests\\") {
        return None;
    }

    if !Path::new("dev-doc").is_dir() {
        return None;
    }

    let doc_root_path = doc_root::resolve("dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return None;
    }

    let phase = yaml::get(&status_file, "phase").ok().flatten().unwrap_or_default();

    if phase != "DEV" && phase != "TEST" {
        Some(format!(
            "[dev-flow] BLOCKED: 当前阶段为 {}，禁止编辑代码文件：{}\n→ 请先完成 {} 阶段文档，进入 DEV 后再编辑代码。",
            phase, file, phase
        ))
    } else {
        None
    }
}

fn is_version_file(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized == "VERSION"
        || normalized.ends_with("/VERSION")
}

fn is_status_file(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized.ends_with("STATUS.yaml")
        && (normalized.starts_with("dev-doc/") || normalized.contains("/dev-doc/"))
}

/// 禁止 agent 直接创建 dev-doc 下应通过 dow doc 创建的文件
fn check_devdoc_direct_create(file: &str) -> Option<String> {
    let normalized = file.replace('\\', "/");

    // 只检查 dev-doc/ 内的文件
    if !normalized.starts_with("dev-doc/") {
        return None;
    }

    // 提取 dev-doc/<branch>/ 之后的相对路径
    let parts: Vec<&str> = normalized.splitn(3, '/').collect();
    if parts.len() < 3 {
        return None;
    }
    let rel = parts[2]; // <branch> 之后的部分

    // 被保护的单文件文档（必须通过 dow doc 创建）
    let protected_singles = [
        ("PRD.md", "prd"),
        ("SPEC.md", "spec"),
        ("TEST.md", "test"),
        ("BRAINSTORM.md", "brainstorm"),
        ("CHANGELOG.md", "changelog"),
    ];

    for (filename, doc_type) in &protected_singles {
        if rel == *filename {
            let path = Path::new(file);
            if !path.exists() {
                return Some(format!(
                    "[dev-flow] BLOCKED: 禁止手动创建 {}，请使用 `dow doc {}`",
                    file, doc_type
                ));
            }
            // 已存在的文件允许编辑（内容填充）
            return None;
        }
    }

    // task/ 和 issue/ 下的新文件（必须通过 dow doc 创建）
    // 匹配标准命名：task_YYYY-MM-DD_N.md / issue_<source>_YYYY-MM-DD_N.md
    if (rel.starts_with("task/task_") || rel.starts_with("issue/issue_"))
        && rel.ends_with(".md")
        && is_standard_doc_filename(rel)
    {
        let path = Path::new(file);
        if !path.exists() {
            let doc_type = if rel.starts_with("task/") { "task" } else { "issue" };
            return Some(format!(
                "[dev-flow] BLOCKED: 禁止手动创建 {}，请使用 `dow doc {} [-n N]`",
                file, doc_type
            ));
        }
    }

    None
}

/// 检查是否为标准 dow doc 命名模式（含日期格式 YYYY-MM-DD）
fn is_standard_doc_filename(rel: &str) -> bool {
    // task_2026-05-29_1.md 或 issue_test_2026-05-29_1.md
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() != 2 {
        return false;
    }
    let filename = parts[1];
    // 检查是否包含日期模式 NNNN-NN-NN
    filename.chars().filter(|c| *c == '-').count() >= 2
        && filename.contains(|c: char| c.is_ascii_digit())
}
