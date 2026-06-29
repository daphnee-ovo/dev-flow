// dow/src/hooks/
// ├── guard.rs  -- dow hooks guard (file write guardian)
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::io::Read as IoRead;
use std::path::{Component, Path, PathBuf};

// ─── GuardPath: normalized absolute path ───────────────────────────────────────────────

struct GuardPath {
    abs: PathBuf,
}

impl GuardPath {
    fn new(raw: &str, project_root: &Path) -> Self {
        let unified = raw.replace('\\', "/");
        let raw_path = Path::new(&unified);

        let joined = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            project_root.join(raw_path)
        };

        let mut parts: Vec<Component> = Vec::new();
        for comp in joined.components() {
            match comp {
                Component::ParentDir => {
                    parts.pop();
                }
                Component::CurDir => {}
                other => parts.push(other),
            }
        }

        GuardPath {
            abs: parts.iter().collect(),
        }
    }

    fn is_under(&self, dir: &Path) -> bool {
        self.abs.starts_with(dir)
    }

    fn is_exact(&self, file: &Path) -> bool {
        self.abs == file
    }

    /// Get relative path from a directory
    fn relative_to(&self, dir: &Path) -> Option<PathBuf> {
        self.abs.strip_prefix(dir).ok().map(|p| p.to_path_buf())
    }

    fn extension(&self) -> Option<&str> {
        self.abs.extension().and_then(|e| e.to_str())
    }

    fn file_name(&self) -> Option<&str> {
        self.abs.file_name().and_then(|n| n.to_str())
    }

    fn exists(&self) -> bool {
        self.abs.exists()
    }

    fn is_dir(&self) -> bool {
        self.abs.is_dir()
    }

    fn display(&self) -> std::path::Display<'_> {
        self.abs.display()
    }
}

// ─── GuardContext: precomputed project paths ──────────────────────────────────────────

struct GuardContext {
    root: PathBuf,
    tmp_dir: PathBuf,
    devdoc_dir: PathBuf,
    docs_dir: PathBuf,
    version_file: PathBuf,
    ai_config_dirs: Vec<PathBuf>,
}

impl GuardContext {
    fn new(root: PathBuf) -> Self {
        let ai_names = [
            ".claude",
            ".codex",
            ".codex-plugin",
            ".agents",
            ".cursor",
            ".aider",
            ".continue",
            ".kiro",
        ];
        let ai_config_dirs: Vec<PathBuf> = ai_names.iter().map(|d| root.join(d)).collect();
        // Special handling for .github/copilot
        let mut dirs = ai_config_dirs;
        dirs.push(root.join(".github/copilot"));

        GuardContext {
            tmp_dir: root.join("tmp"),
            devdoc_dir: root.join(".dev-doc"),
            docs_dir: root.join("docs"),
            version_file: root.join("VERSION"),
            ai_config_dirs: dirs,
            root,
        }
    }

    fn is_ai_config(&self, path: &GuardPath) -> bool {
        self.ai_config_dirs.iter().any(|d| path.is_under(d))
    }

    fn current_branch_dir(&self) -> Option<PathBuf> {
        let branch = doc_root::current_branch()?;
        Some(self.devdoc_dir.join(branch))
    }

    fn read_phase_mode(&self) -> Option<(String, String)> {
        let doc_root_path = doc_root::resolve(self.devdoc_dir.to_str().unwrap_or(".dev-doc"));
        let status_file = doc_root_path.join("STATUS.yaml");
        if !status_file.exists() {
            return None;
        }
        let phase = yaml::get(&status_file, "phase")
            .ok()
            .flatten()
            .unwrap_or_default();
        let mode = yaml::get(&status_file, "mode")
            .ok()
            .flatten()
            .unwrap_or_default();
        Some((phase, mode))
    }

    fn doc_root_path(&self) -> PathBuf {
        doc_root::resolve(self.devdoc_dir.to_str().unwrap_or(".dev-doc"))
    }
}

// ─── Hook output ───────────────────────────────────────────────────────────────

fn deny(reason: &str, kiro_hook: bool) -> Result<i32, DowError> {
    if kiro_hook {
        // kiro-cli: exit code 2 + stderr = block tool execution
        eprintln!("{}", reason);
        return Ok(2);
    }
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

fn ask(reason: &str, kiro_hook: bool) -> Result<i32, DowError> {
    if kiro_hook {
        // kiro-cli: exit code 2 blocks (kiro has no ask intermediate state, only allow/block)
        eprintln!("{}", reason);
        return Ok(2);
    }
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

// ─── Main entry ──────────────────────────────────────────────────────────────────

pub fn run(file: String, kiro_hook: bool) -> Result<i32, DowError> {
    let targets = resolve_targets(&file);
    if targets.is_empty() {
        return Ok(0);
    }

    let project_root = std::env::current_dir().unwrap_or_default();
    let ctx = GuardContext::new(project_root);

    for raw_target in &targets {
        let path = GuardPath::new(raw_target, &ctx.root);

        // 1. Project boundary check
        if !path.is_under(&ctx.root) {
            if is_dangerous_system_path(&path) {
                return deny(&format!("[dev-flow] BLOCKED: writing to system-sensitive path is prohibited: {}", raw_target), kiro_hook);
            }
            return ask(&format!(
                "[dev-flow] Write target is outside project: {}. Please confirm if allowed.",
                raw_target
            ), kiro_hook);
        }

        // 2. VERSION protection
        if path.is_exact(&ctx.version_file) {
            return deny(
                "[dev-flow] BLOCKED: direct modification of VERSION file is prohibited. Use `dow version --set X.Y.Z` or `dow version --bump minor`.",
                kiro_hook,
            );
        }

        // 3. STATUS.yaml protection
        if path.file_name() == Some("STATUS.yaml") && path.is_under(&ctx.devdoc_dir) {
            return deny(
                "[dev-flow] BLOCKED: direct creation or modification of STATUS.yaml is prohibited. Use `dow status set --phase/--mode/--name` or `dow init`.",
                kiro_hook,
            );
        }

        // 4. .dev-doc file creation protection
        if let Some(reason) = check_devdoc_direct_create(&path, &ctx) {
            return deny(&reason, kiro_hook);
        }

        // 5. Cross-branch write protection
        if let Some(reason) = check_cross_branch(&path, &ctx) {
            return deny(&reason, kiro_hook);
        }

        // 6. Phase-based write control
        if let Some(decision) = check_phase_write(&path, &ctx) {
            return match decision {
                PhaseDecision::Deny(reason) => deny(&reason, kiro_hook),
                PhaseDecision::Ask(reason) => ask(&reason, kiro_hook),
            };
        }
    }

    Ok(0)
}

enum PhaseDecision {
    Deny(String),
    Ask(String),
}

// ─── Check functions ────────────────────────────────────────────────────────────────

fn is_dangerous_system_path(path: &GuardPath) -> bool {
    let prefixes: &[&str] = &[
        "/tmp", "/var/tmp", "/dev", "/etc", "/usr", "/bin", "/sbin", "/boot", "/proc", "/sys",
        "/root", "/System", "/Library",
    ];
    prefixes.iter().any(|p| path.is_under(Path::new(p)))
}

fn check_cross_branch(path: &GuardPath, ctx: &GuardContext) -> Option<String> {
    if !path.is_under(&ctx.devdoc_dir) {
        return None;
    }

    let rel = path.relative_to(&ctx.devdoc_dir)?;
    let rel_str = rel.to_string_lossy();

    // Files directly under .dev-doc/ (like archive.db)
    if !rel_str.contains('/') && !rel_str.contains('\\') {
        return None;
    }

    let current = doc_root::current_branch()?;
    let current_branch_dir = ctx.devdoc_dir.join(&current);

    // Under current branch directory → allow
    if path.is_under(&current_branch_dir) {
        return None;
    }

    // Check level by level if belongs to another known branch directory
    let parts: Vec<&str> = rel_str.split('/').collect();
    let mut candidate = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            candidate.push('/');
        }
        candidate.push_str(part);

        if i == parts.len() - 1 {
            break;
        }

        let branch_path = ctx.devdoc_dir.join(&candidate);
        if branch_path.join("STATUS.yaml").exists() {
            return Some(format!(
                "[dev-flow] BLOCKED: current branch is `{}`, writing to another branch's doc directory is prohibited: {}\n→ Please confirm you have switched to the correct branch, or use `git checkout {}` to switch.",
                current, path.display(), candidate
            ));
        }
    }

    None
}

fn check_phase_write(path: &GuardPath, ctx: &GuardContext) -> Option<PhaseDecision> {
    if !ctx.devdoc_dir.is_dir() {
        return None;
    }

    let (phase, mode) = ctx.read_phase_mode()?;

    // DEV/TEST phase
    if phase == "DEV" || phase == "TEST" {
        if mode.starts_with("audit/") {
            return None;
        }
        // Whitelist (always allow)
        if path.is_under(&ctx.devdoc_dir)
            || path.is_under(&ctx.tmp_dir)
            || path.is_under(&ctx.docs_dir)
        {
            return None;
        }
        // AI config dirs: ask (not auto-allow) since they may contain business code
        if ctx.is_ai_config(path) {
            if !has_active_claim(&ctx.doc_root_path()) && has_pending_work(&ctx.doc_root_path()) {
                return Some(PhaseDecision::Ask(format!(
                    "[dev-flow] Writing to AI config path {} without a claim. Please confirm or `dow claim <TASK_ID>` first.",
                    path.display()
                )));
            }
            return None;
        }
        // DEV phase: check agent mismatch (advisory warning, not block)
        if phase == "DEV" {
            if let Some(warning) = check_claim_agent_mismatch(&ctx.doc_root_path()) {
                return Some(PhaseDecision::Ask(warning));
            }
        }
        // DEV phase with no active claim → distinguish reason
        if phase == "DEV" && !has_active_claim(&ctx.doc_root_path()) {
            let doc_root = ctx.doc_root_path();
            let has_undone = has_pending_work(&doc_root);
            if has_undone {
                let expired = crate::core::claim::has_expired_claims(&doc_root);
                let msg = if expired {
                    format!(
                        "[dev-flow] BLOCKED: claim expired, writing to {} not allowed. Please:\n\
                        → `dow claim <TASK_ID>` to re-claim your task",
                        path.display()
                    )
                } else {
                    format!(
                        "[dev-flow] BLOCKED: DEV phase has pending tasks/issues but none claimed, writing to {} not allowed. Please:\n\
                        → `dow claim <TASK_ID>` to claim a task to work on",
                        path.display()
                    )
                };
                return Some(PhaseDecision::Deny(msg));
            } else {
                return Some(PhaseDecision::Deny(format!(
                    "[dev-flow] BLOCKED: DEV phase has no pending tasks or open issues, writing to {} not allowed. Please choose:\n\
                    → `dow task create` to create new task\n\
                    → `dow issue create` to create issue\n\
                    → /test to enter test phase\n\
                    IMPORTANT: Do NOT create tasks/issues and start coding without explicit user approval. Ask the user what they want to do first.",
                    path.display()
                )));
            }
        }
        return None;
    }

    // Non-DEV/TEST phase (PRD/SPEC/TASK etc.)

    // Whitelist 1: tmp (code files need confirmation)
    if path.is_under(&ctx.tmp_dir) {
        if is_code_file(path) {
            return Some(PhaseDecision::Ask(format!(
                "[dev-flow] Current phase is {}, writing code file under tmp/: {}. Confirm this is an exploratory demo?",
                phase,
                path.display()
            )));
        }
        return None;
    }

    // AI config: ask (may contain business code in skills/)
    if ctx.is_ai_config(path) {
        if is_code_file(path) {
            return Some(PhaseDecision::Ask(format!(
                "[dev-flow] Current phase is {}, writing code file under AI config: {}. Confirm?",
                phase, path.display()
            )));
        }
        return None;
    }

    // Whitelist 3: docs (code files prohibited)
    if path.is_under(&ctx.docs_dir) {
        if is_code_file(path) {
            return Some(PhaseDecision::Deny(format!(
                "[dev-flow] BLOCKED: current phase is {}, writing code files under docs/ is not allowed: {}. Code files should be created in DEV phase.",
                phase, path.display()
            )));
        }
        return None;
    }

    // Whitelist 4: .dev-doc workflow files
    if path.is_under(&ctx.devdoc_dir) {
        // STATUS.yaml can only be modified via dow status command
        if path.file_name().map(|f| f == "STATUS.yaml").unwrap_or(false) {
            return Some(PhaseDecision::Deny(format!(
                "[dev-flow] BLOCKED: STATUS.yaml cannot be manually edited, please use `dow status` command."
            )));
        }
        if path.exists() || path.is_dir() {
            return None;
        }
        if is_valid_devdoc_file(path, ctx) {
            return None;
        }
        return Some(PhaseDecision::Deny(format!(
            "[dev-flow] BLOCKED: creating non-workflow files under .dev-doc/ is not allowed: {}. Valid files: PRD.md, SPEC.md, TEST.md, BRAINSTORM.md, CHANGELOG.md, task/task_*.md, issue/issue_*.md, STATUS.yaml",
            path.display()
        )));
    }

    // Everything else → deny
    Some(PhaseDecision::Deny(format!(
        "[dev-flow] BLOCKED: current phase is {}, only .dev-doc/, docs/, and tmp/ writes are allowed. To write to {} please complete planning and enter DEV phase: create task (`dow task create`) or issue (`dow issue create`) to enter DEV. (Exploratory code/demos can go under tmp/)",
        phase, path.display()
    )))
}

fn check_devdoc_direct_create(path: &GuardPath, ctx: &GuardContext) -> Option<String> {
    if !path.is_under(&ctx.devdoc_dir) {
        return None;
    }

    let branch_dir = ctx.current_branch_dir()?;
    let rel = path.relative_to(&branch_dir)?;
    let rel_str = rel.to_string_lossy().to_string();

    // Document-type files: block creation, allow editing existing
    let doc_type_singles = [
        ("PRD.md", "prd"),
        ("SPEC.md", "spec"),
        ("BRAINSTORM.md", "brainstorm"),
    ];

    for (filename, cmd) in &doc_type_singles {
        if rel_str == *filename {
            if !path.exists() {
                return Some(format!(
                    "[dev-flow] BLOCKED: manual creation of {} is prohibited, please use `dow {} create`",
                    path.display(),
                    cmd
                ));
            }
            // Document-type + exists → allow editing
            return None;
        }
    }

    // Structural-type files: always block (create AND edit)
    if rel_str == "CHANGELOG.md" {
        return Some(format!(
            "[dev-flow] BLOCKED: direct modification of CHANGELOG.md is prohibited. Use `dow changelog add --text \"...\"`"
        ));
    }

    // Task files: always block
    if rel_str.starts_with("task/") && rel_str.ends_with(".md") {
        let name = rel_str.split('/').last().unwrap_or("");
        if name.starts_with("task_") || name.starts_with("done_task_") {
            if !path.exists() {
                return Some(format!(
                    "[dev-flow] BLOCKED: manual creation of task files is prohibited, please use `dow task create`"
                ));
            }
            return Some(format!(
                "[dev-flow] BLOCKED: direct modification of task files is prohibited. Use `dow task done <ID>` or `dow task reopen <ID>`"
            ));
        }
    }

    // Issue files: always block
    if rel_str.starts_with("issue/") && rel_str.ends_with(".md") {
        let name = rel_str.split('/').last().unwrap_or("");
        if name.starts_with("issue_") || name.starts_with("closed_issue_") {
            if !path.exists() {
                return Some(format!(
                    "[dev-flow] BLOCKED: manual creation of issue files is prohibited, please use `dow issue create`"
                ));
            }
            return Some(format!(
                "[dev-flow] BLOCKED: direct modification of issue files is prohibited. Use `dow issue close <ID>` or `dow issue reopen <ID>`"
            ));
        }
    }

    None
}

// ─── Helper functions ────────────────────────────────────────────────────────────────

fn has_active_claim(doc_root: &Path) -> bool {
    !crate::core::claim::get_active_claims(doc_root).is_empty()
}

/// Check if the current agent matches the claim owner; returns warning message if mismatch
fn check_claim_agent_mismatch(doc_root: &Path) -> Option<String> {
    let claim_agent = crate::core::claim::get_claim_agent_id(doc_root)?;
    let current_agent = crate::core::claim::detect_agent_id()?;
    if claim_agent != current_agent {
        Some(format!(
            "[dev-flow] WARNING: another agent ({}) holds the claim. You may be modifying files owned by a different session.",
            claim_agent
        ))
    } else {
        None
    }
}

fn has_pending_work(doc_root: &Path) -> bool {
    let task_dir = doc_root.join("task");
    if crate::core::task_store::has_active_work(&task_dir) {
        return true;
    }
    let issue_dir = doc_root.join("issue");
    if issue_dir.is_dir() {
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
    }
    false
}

fn is_code_file(path: &GuardPath) -> bool {
    matches!(
        path.extension(),
        Some(
            "py" | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "rs"
                | "go"
                | "java"
                | "rb"
                | "php"
                | "vue"
                | "svelte"
                | "sh"
                | "bash"
                | "zsh"
                | "cpp"
                | "c"
                | "h"
                | "hpp"
                | "cc"
                | "cxx"
                | "kt"
                | "kts"
                | "swift"
                | "dart"
                | "m"
                | "mm"
                | "scala"
                | "lua"
                | "sql"
                | "pl"
                | "pm"
                | "r"
                | "cs"
                | "fs"
                | "ex"
                | "exs"
                | "erl"
                | "zig"
                | "nim"
                | "ps1"
                | "psm1"
                | "bat"
                | "cmd"
        )
    )
}

fn is_valid_devdoc_file(path: &GuardPath, ctx: &GuardContext) -> bool {
    let branch_dir = match ctx.current_branch_dir() {
        Some(d) => d,
        None => return false,
    };
    let rel = match path.relative_to(&branch_dir) {
        Some(r) => r,
        None => return false,
    };
    let rel_str = rel.to_string_lossy().to_string();

    let valid_singles = [
        "PRD.md",
        "SPEC.md",
        "TEST.md",
        "BRAINSTORM.md",
        "CHANGELOG.md",
        "STATUS.yaml",
    ];
    if valid_singles.contains(&rel_str.as_str()) {
        return true;
    }

    if rel_str.starts_with("task/") {
        let filename = &rel_str[5..];
        return (filename.starts_with("task_") || filename.starts_with("done_task_"))
            && filename.ends_with(".md");
    }

    if rel_str.starts_with("issue/") {
        let filename = &rel_str[6..];
        return (filename.starts_with("issue_") || filename.starts_with("closed_issue_"))
            && filename.ends_with(".md");
    }

    false
}


// ─── Input parsing (unchanged) ────────────────────────────────────────────────────────

fn resolve_targets(file: &str) -> Vec<String> {
    if !file.is_empty() {
        if let Ok(content) = std::fs::read_to_string(file) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return extract_targets_from_json(&json);
            }
        }
        return vec![file.to_string()];
    }

    let mut stdin_buf = String::new();
    if std::io::stdin().read_to_string(&mut stdin_buf).is_err() || stdin_buf.is_empty() {
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

    extract_targets_from_json(&json)
}

fn extract_targets_from_json(json: &serde_json::Value) -> Vec<String> {
    let tool_input = json.get("tool_input").unwrap_or(json);
    let tool_name = json.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");

    match tool_name {
        "Write" | "Edit" | "fs_write" => {
            if let Some(path) = tool_input
                .get("file_path")
                .or_else(|| tool_input.get("path"))
                .and_then(|v| v.as_str())
            {
                vec![path.to_string()]
            } else {
                vec![]
            }
        }
        "Bash" | "execute_bash" => {
            let command = tool_input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            extract_write_targets_from_command(&command)
        }
        _ => {
            if let Some(path) = tool_input
                .get("file_path")
                .or_else(|| tool_input.get("path"))
                .and_then(|v| v.as_str())
            {
                vec![path.to_string()]
            } else if let Some(cmd) = tool_input.get("command").and_then(|v| v.as_str()) {
                extract_write_targets_from_command(cmd)
            } else {
                vec![]
            }
        }
    }
}

fn extract_write_targets_from_command(cmd: &str) -> Vec<String> {
    let mut targets = Vec::new();

    // Redirect write: > file, >> file
    let redirect_parts: Vec<&str> = cmd.split('>').collect();
    for (i, part) in redirect_parts.iter().enumerate().skip(1) {
        if part.is_empty() {
            continue;
        }
        let prev = redirect_parts[i - 1];
        let prev_trimmed = prev.trim_end();
        if prev_trimmed.ends_with('&') {
            continue;
        }
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

    // tee write
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

    // cp/mv target
    for prefix in &["cp ", "mv "] {
        if let Some(pos) = cmd.find(prefix) {
            let args_str = &cmd[pos + prefix.len()..];
            let args_end = args_str
                .find(';')
                .or_else(|| args_str.find("&&"))
                .or_else(|| args_str.find('|'))
                .unwrap_or(args_str.len());
            let tokens: Vec<&str> = args_str[..args_end]
                .split_whitespace()
                .filter(|t| !t.starts_with('-'))
                .collect();
            if let Some(dest) = tokens.last() {
                let clean = dest.trim_matches('"').trim_matches('\'');
                if looks_like_path(clean) {
                    targets.push(clean.to_string());
                }
            }
        }
    }

    // sed -i / perl -i
    for prefix in &["sed ", "perl "] {
        if let Some(pos) = cmd.find(prefix) {
            let args_str = &cmd[pos + prefix.len()..];
            let tokens: Vec<&str> = args_str.split_whitespace().collect();
            let has_inplace = tokens.iter().any(|t| {
                *t == "-i"
                    || t.starts_with("-i.")
                    || t.starts_with("-i'")
                    || *t == "-pi"
                    || *t == "-pie"
            });
            if has_inplace {
                if let Some(last) = tokens.last() {
                    let clean = last.trim_matches('"').trim_matches('\'');
                    if looks_like_path(clean) {
                        targets.push(clean.to_string());
                    }
                }
            }
        }
    }

    // dd of=<file>
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
    let protected_names = ["VERSION", "Makefile", "Dockerfile", "CHANGELOG"];
    if protected_names.contains(&s) {
        return true;
    }
    s.contains('/') || s.contains('.') || s.starts_with(".dev-doc")
}
