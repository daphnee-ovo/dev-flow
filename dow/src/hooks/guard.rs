// dow/src/hooks/
// ├── guard.rs  -- dow hooks guard（文件写入守护）
//    合并 block-system-tmp.sh + block-non-dev-edit.sh

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::io::Read as IoRead;
use std::path::Path;

/// 输出 Claude Code PreToolUse deny JSON 并返回 exit 0
/// Claude Code 要求 exit 0 + JSON permissionDecision:"deny" 才能阻断工具执行
fn deny(reason: &str) -> Result<i32, DowError> {
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    });
    println!("{}", output);
    Ok(0)
}

/// 输出 ask JSON，触发用户权限确认（非危险但超出项目范围的写入）
fn ask(reason: &str) -> Result<i32, DowError> {
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": reason
        }
    });
    println!("{}", output);
    Ok(0)
}

pub fn run(file: String) -> Result<i32, DowError> {
    // 收集需要检查的文件路径列表
    let targets = resolve_targets(&file);

    if targets.is_empty() {
        return Ok(0);
    }

    let project_root = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    for target in &targets {
        // 路径穿越检测：resolve 后判断是否在项目目录内
        let resolved = resolve_absolute(target);
        if !is_within_project(&resolved) {
            // 危险系统路径 → 直接 deny
            if is_dangerous_path(&resolved) {
                return deny(&format!(
                    "[dev-flow] 禁止写入系统敏感路径：{}",
                    target
                ));
            }
            // 非危险项目外路径 → 触发权限确认
            return ask(&format!(
                "[dev-flow] 写入目标在项目外：{}。请确认是否允许。",
                target
            ));
        }

        // 将绝对路径转为相对路径（后续检查统一使用相对路径）
        let rel_target = if target.starts_with(&project_root) {
            target[project_root.len()..].trim_start_matches('/').to_string()
        } else {
            target.to_string()
        };
        let rel_target = rel_target.as_str();

        // block-version-direct-write: 禁止 agent 直接修改 VERSION 文件
        if is_version_file(rel_target) {
            return deny(
                "[dev-flow] 禁止直接修改 VERSION 文件。请使用 `dow version --set X.Y.Z` 或 `dow version --bump minor`。"
            );
        }

        // block-status-direct-write: 禁止 agent 直接创建/修改 STATUS.yaml
        if is_status_file(rel_target) {
            return deny(
                "[dev-flow] 禁止直接创建或修改 STATUS.yaml。请使用 `dow status --phase/--mode/--name` 或 `dow init`。"
            );
        }

        // block-devdoc-direct-create: 禁止 agent 手动创建 .dev-doc 文档文件
        if let Some(msg) = check_devdoc_direct_create(rel_target) {
            return deny(&msg);
        }

        // block-cross-branch: 拦截写入其他分支的 .dev-doc 目录
        if let Some(reason) = check_cross_branch_write(rel_target) {
            return deny(&reason);
        }

        // 非 DEV/TEST 阶段：只允许写入 .dev-doc/<branch>/ 已存在文件和 tmp/
        if let Some(reason) = check_non_dev_write(rel_target) {
            return deny(&reason);
        }
    }

    Ok(0)
}

/// 从 stdin（Claude Code hook JSON）中读取 tool_input，提取写入目标
/// Claude Code hook 通过 stdin 传入 JSON: {"tool_name":"...", "tool_input":{...}}
fn resolve_targets(file: &str) -> Vec<String> {
    // 如果命令行传入了路径（兼容旧调用方式），直接使用
    if !file.is_empty() {
        return vec![file.to_string()];
    }

    // 从 stdin 读取 hook JSON
    let mut stdin_buf = String::new();
    if std::io::stdin().read_to_string(&mut stdin_buf).is_err() || stdin_buf.is_empty() {
        // fallback: 尝试环境变量（向后兼容测试场景）
        let tool_input = std::env::var("TOOL_INPUT").unwrap_or_default();
        if tool_input.is_empty() {
            return vec![];
        }
        stdin_buf = tool_input;
    }

    let json: serde_json::Value = match serde_json::from_str(&stdin_buf) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let tool_input = json.get("tool_input").unwrap_or(&json);
    let tool_name = json.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");

    match tool_name {
        "Write" | "Edit" => {
            // tool_input.file_path 是目标文件
            if let Some(path) = tool_input.get("file_path").and_then(|v| v.as_str()) {
                vec![path.to_string()]
            } else {
                vec![]
            }
        }
        "Bash" => {
            // tool_input.command 是 bash 命令，解析写入目标
            let command = tool_input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            extract_write_targets_from_command(&command)
        }
        _ => {
            // 未知工具或旧格式：尝试 file_path 或 command
            if let Some(path) = tool_input.get("file_path").and_then(|v| v.as_str()) {
                vec![path.to_string()]
            } else if let Some(cmd) = tool_input.get("command").and_then(|v| v.as_str()) {
                extract_write_targets_from_command(cmd)
            } else {
                vec![]
            }
        }
    }
}

/// 从 Bash 命令中提取可能的写入目标路径
fn extract_write_targets_from_command(cmd: &str) -> Vec<String> {
    let mut targets = Vec::new();

    // 匹配重定向写入：> file、>> file
    // 跳过第一个片段（> 之前的命令部分），排除 fd redirect (2>, &>)
    let redirect_parts: Vec<&str> = cmd.split('>').collect();
    for (i, part) in redirect_parts.iter().enumerate().skip(1) {
        if part.is_empty() {
            continue;
        }
        // 检查 > 前是否为 fd redirect（如 2>/dev/null、1>/dev/null、&>/dev/null）
        let prev = redirect_parts[i - 1];
        let prev_trimmed = prev.trim_end();
        if prev_trimmed.ends_with('&') {
            continue;
        }
        // fd redirect: > 前紧邻数字，且该数字前为空白或行首
        if let Some(last_char) = prev_trimmed.chars().last() {
            if last_char.is_ascii_digit() {
                let before_digit = &prev_trimmed[..prev_trimmed.len() - 1];
                if before_digit.is_empty() || before_digit.ends_with(char::is_whitespace) {
                    continue;
                }
            }
        }
        let after = part.trim_start_matches('>').trim();
        if let Some(path) = after.split_whitespace().next() {
            let clean = path.trim_matches('"').trim_matches('\'');
            if clean == "/dev/null" {
                continue;
            }
            if looks_like_path(clean) {
                targets.push(clean.to_string());
            }
        }
    }

    // 匹配 tee 写入：| tee file、| tee -a file
    if cmd.contains("tee") {
        let segments: Vec<&str> = cmd.split("tee").collect();
        for segment in segments.iter().skip(1) {
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

    // 匹配 sed -i / perl -i 原地修改：最后一个非 flag 参数是目标文件
    for prefix in &["sed ", "perl "] {
        if let Some(pos) = cmd.find(prefix) {
            let args_str = &cmd[pos + prefix.len()..];
            // 检查是否含 -i 标志（原地修改）
            let tokens: Vec<&str> = args_str.split_whitespace().collect();
            let has_inplace = tokens.iter().any(|t| *t == "-i" || t.starts_with("-i.") || t.starts_with("-i'") || *t == "-pi" || *t == "-pie");
            if has_inplace {
                // 最后一个非 flag、非表达式的 token 通常是文件
                if let Some(last) = tokens.last() {
                    let clean = last.trim_matches('"').trim_matches('\'');
                    if looks_like_path(clean) {
                        targets.push(clean.to_string());
                    }
                }
            }
        }
    }

    // 匹配 dd of=<file>
    if cmd.contains("dd ") || cmd.starts_with("dd ") {
        for token in cmd.split_whitespace() {
            if let Some(stripped) = token.strip_prefix("of=") {
                let clean = stripped.trim_matches('"').trim_matches('\'');
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
    // 已知受保护的无扩展名文件
    let protected_names = ["VERSION", "Makefile", "Dockerfile", "CHANGELOG"];
    if protected_names.contains(&s) {
        return true;
    }
    s.contains('/') || s.contains('.') || s.starts_with(".dev-doc")
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

/// 危险系统路径：直接 deny，不允许用户 override
fn is_dangerous_path(resolved: &str) -> bool {
    let prefixes = [
        "/tmp/", "/var/tmp/", "/dev/", "/etc/", "/usr/",
        "/bin/", "/sbin/", "/boot/", "/proc/", "/sys/",
        "/root/", "/System/", "/Library/",
    ];
    prefixes.iter().any(|p| resolved.starts_with(p))
}

fn check_cross_branch_write(file: &str) -> Option<String> {
    // 只检查 .dev-doc/ 内的写入
    let normalized = file.replace('\\', "/");
    if !normalized.starts_with(".dev-doc/") {
        return None;
    }

    let rest = &normalized[".dev-doc/".len()..];

    // 直接在 .dev-doc/ 下的文件（如 archive.db）不属于分支目录
    if !rest.contains('/') {
        return None;
    }

    // 获取当前分支（分支名可能含 `/`，如 refactor/tui）
    let current = doc_root::current_branch()?;
    let current_prefix = format!(".dev-doc/{}/", current);

    // 文件在当前分支目录下 → 允许
    if normalized.starts_with(&current_prefix) {
        return None;
    }

    // 不在当前分支前缀下 → 逐级检查是否属于其他已知分支目录（含 STATUS.yaml）
    let parts: Vec<&str> = rest.split('/').collect();
    let mut candidate = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            candidate.push('/');
        }
        candidate.push_str(part);

        // 最后一段可能是文件名，不检查
        if i == parts.len() - 1 {
            break;
        }

        let branch_path = Path::new(".dev-doc").join(&candidate);
        if branch_path.join("STATUS.yaml").exists() {
            return Some(format!(
                "[dev-flow] BLOCKED: 当前分支为 `{}`，禁止写入其他分支的文档目录：{}\n→ 请确认你已切换到正确的分支，或使用 `git checkout {}` 切换。",
                current, file, candidate
            ));
        }
    }

    // 未命中任何已知分支目录 → 允许（可能是新分支初始化等情况）
    None
}

/// 非 DEV/TEST 阶段写入白名单检查
/// 允许：.dev-doc/<current-branch>/ 已存在文件、项目内 tmp/
/// 其余位置一律 deny
fn check_non_dev_write(file: &str) -> Option<String> {
    if !Path::new(".dev-doc").is_dir() {
        return None;
    }

    let doc_root_path = doc_root::resolve(".dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return None;
    }

    let phase = yaml::get(&status_file, "phase").ok().flatten().unwrap_or_default();

    // DEV/TEST 阶段不限制
    if phase == "DEV" || phase == "TEST" {
        return None;
    }

    // 白名单 1：项目内 tmp/ 目录
    if file.starts_with("tmp/") || file.starts_with("tmp\\") {
        return None;
    }

    // 白名单 2：AI 工具配置目录
    let ai_config_prefixes = [
        ".claude/", ".codex/", ".codex-plugin/",
        ".agents/", ".cursor/", ".github/copilot/",
        ".aider/", ".continue/",
    ];
    if ai_config_prefixes.iter().any(|p| file.starts_with(p)) {
        return None;
    }

    // 白名单 3：任意层级 docs/ 目录（docs/、xxx/docs/、...）
    if is_docs_path(file) {
        return None;
    }

    // 白名单 4：.dev-doc/<branch>/ 内的合法工作流文件
    if file.starts_with(".dev-doc/") || file.starts_with(".dev-doc\\") {
        // 已存在的文件允许编辑（新建文件已被 check_devdoc_direct_create 拦截）
        if Path::new(file).exists() {
            return None;
        }
        // .dev-doc 下的目录本身允许
        if Path::new(file).is_dir() {
            return None;
        }
        // 新文件：检查是否属于 dev-flow 工作流管理范围
        if is_valid_devdoc_file(file) {
            return None;
        }
        return Some(format!(
            "[dev-flow] .dev-doc/ 下不允许创建非工作流文件：{}。合法文件：PRD.md、SPEC.md、TEST.md、BRAINSTORM.md、CHANGELOG.md、task/task_*.md、issue/issue_*.md、STATUS.yaml",
            file
        ));
    }

    // 其余位置：非 DEV 阶段禁止写入
    Some(format!(
        "[dev-flow] 当前阶段为 {}，只允许写入 .dev-doc/、docs/ 和 tmp/。要写入 {} 请先完成规划并进入 DEV 阶段：创建任务（/task）或创建 issue（/issue）后即可进入 DEV。（探索性代码、demo 可放 tmp/ 下）",
        phase, file
    ))
}

fn is_version_file(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized == "VERSION"
        || normalized.ends_with("/VERSION")
}

fn is_status_file(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized.ends_with("STATUS.yaml")
        && (normalized.starts_with(".dev-doc/") || normalized.contains("/.dev-doc/"))
}

/// 禁止 agent 直接创建 .dev-doc 下应通过 dow doc 创建的文件
fn check_devdoc_direct_create(file: &str) -> Option<String> {
    let normalized = file.replace('\\', "/");

    // 只检查 .dev-doc/ 内的文件
    if !normalized.starts_with(".dev-doc/") {
        return None;
    }

    // 提取 .dev-doc/<branch>/ 之后的相对路径
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

/// 判断路径是否包含 docs/ 段（任意层级）
fn is_docs_path(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized.starts_with("docs/") || normalized.contains("/docs/")
}

/// 判断 .dev-doc/ 内新文件是否属于 dev-flow 工作流管理范围
fn is_valid_devdoc_file(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    // 提取 .dev-doc/<branch>/ 之后的相对路径
    let parts: Vec<&str> = normalized.splitn(3, '/').collect();
    if parts.len() < 3 {
        return false;
    }
    let rel = parts[2];

    // 合法的顶层文档
    let valid_singles = [
        "PRD.md", "SPEC.md", "TEST.md", "BRAINSTORM.md", "CHANGELOG.md", "STATUS.yaml",
    ];
    if valid_singles.contains(&rel) {
        return true;
    }

    // task/ 下：task_YYYY-MM-DD_N.md 或 done_task_YYYY-MM-DD_N.md
    if rel.starts_with("task/") {
        let filename = &rel[5..];
        return (filename.starts_with("task_") || filename.starts_with("done_task_"))
            && filename.ends_with(".md");
    }

    // issue/ 下：issue_*_YYYY-MM-DD_N.md 或 closed_issue_*_YYYY-MM-DD_N.md
    if rel.starts_with("issue/") {
        let filename = &rel[6..];
        return (filename.starts_with("issue_") || filename.starts_with("closed_issue_"))
            && filename.ends_with(".md");
    }

    false
}
