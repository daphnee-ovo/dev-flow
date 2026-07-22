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
                return deny(
                    &format!(
                        "[dev-flow] BLOCKED: writing to system-sensitive path is prohibited: {}",
                        raw_target
                    ),
                    kiro_hook,
                );
            }
            return ask(
                &format!(
                    "[dev-flow] Write target is outside project: {}. Please confirm if allowed.",
                    raw_target
                ),
                kiro_hook,
            );
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
        // TEST phase: deny all source code writes
        if phase == "TEST" {
            return Some(PhaseDecision::Deny(format!(
                "[dev-flow] BLOCKED: TEST phase only allows test execution, not source code modification. If a fix is needed:\n\
                → `dow status set --phase DEV` to switch to DEV phase\n\
                → `dow task create` or `dow issue create` to track the work\n\
                Attempted write: {}",
                path.display()
            )));
        }
        // DEV phase: check agent mismatch (advisory warning, not block)
        if phase == "DEV" {
            if let Some(warning) = check_claim_agent_mismatch(&ctx.doc_root_path()) {
                return Some(PhaseDecision::Ask(warning));
            }
        }
        // DEV phase with active claim: check file scope
        if phase == "DEV" && has_active_claim(&ctx.doc_root_path()) {
            if let Some(warning) = check_claim_file_scope(&ctx.doc_root_path(), &ctx.root, path) {
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
                    → `dow status set --phase <PHASE>` to switch phase (PRD/SPEC/TASK/TEST/ITERATE)\n\
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
                phase,
                path.display()
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
        if path
            .file_name()
            .map(|f| f == "STATUS.yaml")
            .unwrap_or(false)
        {
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
        "[dev-flow] BLOCKED: current phase is {}, only .dev-doc/, docs/, and tmp/ writes are allowed. To write to {} please complete planning and enter DEV phase:\n\
        → `dow task create` to create task and enter DEV\n\
        → `dow issue create` to create issue and enter DEV\n\
        → `dow status set --phase DEV` to switch to DEV phase directly\n\
        (Exploratory code/demos can go under tmp/)",
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

fn check_claim_file_scope(
    doc_root: &Path,
    project_root: &Path,
    path: &GuardPath,
) -> Option<String> {
    let claimed_ids = crate::core::claim::get_active_claims(doc_root);
    if claimed_ids.is_empty() {
        return None;
    }

    let all_tasks = crate::commands::task::get_all_task_details(doc_root);
    let mut allowed_files: Vec<String> = Vec::new();

    for cid in &claimed_ids {
        let full_id = crate::core::item_id::normalize_full(cid);
        if let Some(parsed) = crate::core::item_id::parse(cid) {
            match parsed.kind {
                crate::core::item_id::ItemKind::Task => {
                    if let Some(task) = all_tasks.iter().find(|t| t.id == full_id) {
                        for f in task
                            .files
                            .create
                            .iter()
                            .chain(task.files.modify.iter())
                            .chain(task.files.test.iter())
                        {
                            if !f.is_empty() {
                                allowed_files.push(f.clone());
                            }
                        }
                    }
                }
                crate::core::item_id::ItemKind::Issue => {
                    let issue_files = get_issue_files(doc_root, &full_id);
                    allowed_files.extend(issue_files);
                }
            }
        }
    }

    // If no files declared in any claimed item, skip scope check
    if allowed_files.is_empty() {
        return None;
    }

    // Get relative path from project root
    let rel_path = path.relative_to(project_root)?;
    let rel_str = rel_path.to_string_lossy();

    let in_scope = allowed_files
        .iter()
        .any(|f| rel_str.ends_with(f.as_str()) || *rel_str == *f);

    if in_scope {
        None
    } else {
        Some(format!(
            "[dev-flow] WARNING: writing to {} which is outside claimed task's declared files.\n\
            → Consider `dow task update <ID> --file '{{\"modify\":[\"{}\"]}}'` to declare this file.",
            path.display(),
            rel_str
        ))
    }
}

fn get_issue_files(doc_root: &Path, target_id: &str) -> Vec<String> {
    let issue_dir = doc_root.join("issue");
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(&issue_dir) {
        Ok(e) => e,
        Err(_) => return files,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") || name.starts_with("closed_") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            let mut in_target = false;
            for line in content.lines() {
                if line.starts_with("- [ ]") && line.contains(target_id) {
                    in_target = true;
                } else if in_target && (line.starts_with("- [ ]") || line.starts_with("- [x]")) {
                    break;
                } else if in_target {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("- files_modify:") {
                        files.extend(crate::commands::task::parse_inline_list(rest));
                    } else if let Some(rest) = trimmed.strip_prefix("- files_create:") {
                        files.extend(crate::commands::task::parse_inline_list(rest));
                    }
                }
            }
        }
    }
    files
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
        // Some integrations pass the Bash command as the positional guard
        // argument instead of using hook JSON on stdin. Classify it before
        // treating path-looking metadata values as a write target.
        if is_trusted_flow_command(file) {
            return vec![];
        }
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
            extract_command_targets(&command)
        }
        _ if tool_name.eq_ignore_ascii_case("apply_patch") => {
            extract_apply_patch_targets(tool_input)
        }
        _ => {
            if let Some(path) = tool_input
                .get("file_path")
                .or_else(|| tool_input.get("path"))
                .and_then(|v| v.as_str())
            {
                vec![path.to_string()]
            } else if let Some(cmd) = tool_input.get("command").and_then(|v| v.as_str()) {
                extract_command_targets(cmd)
            } else {
                vec![]
            }
        }
    }
}

/// Extract file targets from an apply_patch payload without interpreting patch
/// body text as shell syntax. Only the patch format's file marker lines are
/// write targets.
fn extract_apply_patch_targets(tool_input: &serde_json::Value) -> Vec<String> {
    let patch = tool_input
        .as_str()
        .or_else(|| tool_input.get("patch").and_then(|v| v.as_str()))
        .or_else(|| tool_input.get("input").and_then(|v| v.as_str()))
        .or_else(|| tool_input.get("command").and_then(|v| v.as_str()))
        .unwrap_or_default();
    let markers = ["*** Update File:", "*** Add File:", "*** Delete File:"];

    patch
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            markers
                .iter()
                .find_map(|marker| line.strip_prefix(marker).map(str::trim))
        })
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

/// Extract actual write targets from a Bash hook command while keeping dev-flow
/// metadata arguments out of the path scanner.
fn extract_command_targets(cmd: &str) -> Vec<String> {
    // A trusted flow-management command may contain values such as
    // `src/example.rs` in --file, refs, or stdin JSON.
    // Those values describe the task; they are not writes performed by Bash.
    // Classify this command before scanning metadata so prose cannot create
    // fake targets. Shell operators make the command ineligible for trust and
    // are handled by the write scanner below.
    if is_trusted_flow_command(cmd) {
        return vec![];
    }

    extract_write_targets_from_command(cmd)
}

/// Return true only for the trusted `dow` executable and known metadata
/// subcommands. A pipeline is accepted only when its input is produced by
/// `echo` or `printf` and the final command is trusted `dow`.
fn is_trusted_flow_command(cmd: &str) -> bool {
    let segments = match split_shell_pipeline(cmd) {
        Some(segments) if !segments.is_empty() => segments,
        _ => return false,
    };

    if segments.len() == 1 {
        return is_trusted_dow_invocation(&segments[0]);
    }

    if segments.len() != 2 {
        return false;
    }

    let input_tokens = shell_words(&segments[0]);
    let input_command = input_tokens.first().map(String::as_str);
    if !matches!(input_command, Some("echo") | Some("printf")) {
        return false;
    }

    is_trusted_dow_invocation(&segments[1])
}

fn is_trusted_dow_invocation(segment: &str) -> bool {
    let tokens = shell_words(segment);
    let Some(executable) = tokens.first() else {
        return false;
    };
    if !is_trusted_dow_executable(executable) {
        return false;
    }

    let args: Vec<&str> = tokens
        .iter()
        .skip(1)
        .map(String::as_str)
        .filter(|token| *token != "-H" && *token != "--human")
        .collect();

    match args.as_slice() {
        ["status", ..] | ["claim", ..] | ["init", ..] | ["version", ..] => true,
        ["task", subcommand, ..] => matches!(
            *subcommand,
            "create" | "update" | "done" | "remove" | "reopen" | "list" | "show" | "schema"
        ),
        ["issue", subcommand, ..] => matches!(
            *subcommand,
            "create" | "update" | "close" | "remove" | "reopen" | "list" | "show" | "schema"
        ),
        ["changelog", subcommand, ..] => matches!(*subcommand, "add" | "list"),
        _ => false,
    }
}

fn is_trusted_dow_executable(executable: &str) -> bool {
    if executable == "dow" {
        return true;
    }

    let current_executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let command_path = Path::new(executable);
    let command_path = if command_path.is_absolute() {
        command_path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(current_dir) => current_dir.join(command_path),
            Err(_) => return false,
        }
    };

    match (
        std::fs::canonicalize(command_path),
        std::fs::canonicalize(current_executable),
    ) {
        (Ok(command_path), Ok(current_executable)) => command_path == current_executable,
        _ => false,
    }
}

/// Split a command on a single, unquoted pipe. Other shell operators make the
/// command ineligible for the exemption and remain handled by the write scan.
fn split_shell_pipeline(cmd: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    let chars: Vec<char> = cmd.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            current.push(ch);
            escaped = true;
            index += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            if active_quote == '"' && ch == '$' && chars.get(index + 1) == Some(&'(') {
                return None;
            }
            if active_quote != '\'' && ch == '`' {
                return None;
            }
            current.push(ch);
            if ch == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            current.push(ch);
            index += 1;
            continue;
        }
        if ch == '|' {
            if chars.get(index + 1) == Some(&'|') {
                return None;
            }
            segments.push(current.trim().to_string());
            current.clear();
            index += 1;
            continue;
        }
        if matches!(ch, ';' | '&' | '<' | '>' | '`')
            || (ch == '$' && chars.get(index + 1) == Some(&'('))
        {
            return None;
        }
        current.push(ch);
        index += 1;
    }

    if quote.is_some() || escaped {
        return None;
    }
    segments.push(current.trim().to_string());
    if segments.iter().any(String::is_empty) {
        return None;
    }
    Some(segments)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShellToken {
    Word(String),
    Pipe,
    And,
    Or,
    Semicolon,
    Redirect { append: bool },
    InputRedirect,
}

fn flush_shell_word(tokens: &mut Vec<ShellToken>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(ShellToken::Word(std::mem::take(current)));
    }
}

/// Tokenize shell syntax without treating operators inside quoted arguments as
/// shell operators. Unsupported shell constructs return `None` so callers can
/// conservatively keep the command guarded instead of silently allowing it.
fn tokenize_shell(cmd: &str) -> Option<Vec<ShellToken>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let chars: Vec<char> = cmd.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && ch == '$' && chars.get(index + 1) == Some(&'(') {
                return None;
            }
            if active_quote != '\'' && ch == '`' {
                return None;
            }
            if ch == active_quote {
                quote = None;
                index += 1;
                continue;
            }
            if ch == '\\' && active_quote != '\'' {
                escaped = true;
                index += 1;
                continue;
            }
            current.push(ch);
            index += 1;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            index += 1;
            continue;
        }
        if ch.is_whitespace() {
            flush_shell_word(&mut tokens, &mut current);
            index += 1;
            continue;
        }

        match ch {
            '|' => {
                flush_shell_word(&mut tokens, &mut current);
                if chars.get(index + 1) == Some(&'|') {
                    tokens.push(ShellToken::Or);
                    index += 2;
                } else {
                    tokens.push(ShellToken::Pipe);
                    index += 1;
                }
            }
            '&' => {
                if chars.get(index + 1) == Some(&'&') {
                    flush_shell_word(&mut tokens, &mut current);
                    tokens.push(ShellToken::And);
                    index += 2;
                } else if matches!(tokens.last(), Some(ShellToken::Redirect { .. })) {
                    // Handle the common `2>&1`/`2>&2` descriptor form as a
                    // redirect target that should not become a filesystem path.
                    current.push(ch);
                    index += 1;
                    while chars.get(index).is_some_and(char::is_ascii_digit) {
                        current.push(chars[index]);
                        index += 1;
                    }
                } else {
                    return None;
                }
            }
            ';' => {
                flush_shell_word(&mut tokens, &mut current);
                tokens.push(ShellToken::Semicolon);
                index += 1;
            }
            '>' => {
                flush_shell_word(&mut tokens, &mut current);
                let append = chars.get(index + 1) == Some(&'>');
                tokens.push(ShellToken::Redirect { append });
                index += if append { 2 } else { 1 };
            }
            '<' => {
                if chars.get(index + 1) == Some(&'<') {
                    return None;
                }
                flush_shell_word(&mut tokens, &mut current);
                tokens.push(ShellToken::InputRedirect);
                index += 1;
            }
            '`' => return None,
            '$' if chars.get(index + 1) == Some(&'(') => return None,
            '(' | ')' => return None,
            _ => {
                current.push(ch);
                index += 1;
            }
        }
    }

    if quote.is_some() || escaped {
        return None;
    }
    flush_shell_word(&mut tokens, &mut current);
    Some(tokens)
}

fn shell_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in segment.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escaped || quote.is_some() {
        return vec![];
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn extract_write_targets_from_command(cmd: &str) -> Vec<String> {
    let Some(tokens) = tokenize_shell(cmd) else {
        return vec![cmd.to_string()];
    };

    extract_write_targets_from_tokens(&tokens)
}

fn extract_write_targets_from_tokens(tokens: &[ShellToken]) -> Vec<String> {
    let mut targets = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token, ShellToken::Redirect { .. }) {
            continue;
        }
        let Some(ShellToken::Word(target)) = tokens.get(index + 1) else {
            continue;
        };
        if !matches!(target.as_str(), "/dev/null" | "&1" | "&2") {
            targets.push(target.clone());
        }
    }

    for segment in shell_command_segments(tokens) {
        let words = simple_command_words(segment);
        let Some(command_index) = find_command_index(&words) else {
            continue;
        };
        let command = executable_name(&words[command_index]);
        let args = &words[command_index + 1..];

        match command {
            "tee" => {
                if let Some(target) = first_positional_arg(args) {
                    if target != "/dev/null" {
                        targets.push(target.to_string());
                    }
                }
            }
            "cp" | "mv" => {
                if let Some(target) = positional_args(args).last() {
                    if *target != "/dev/null" {
                        targets.push((*target).to_string());
                    }
                }
            }
            "sed" | "perl" => {
                let has_inplace = args.iter().any(|arg| {
                    arg == "-i"
                        || arg.starts_with("-i.")
                        || arg.starts_with("-i'")
                        || arg == "-pi"
                        || arg == "-pie"
                });
                if has_inplace {
                    if let Some(target) = args.last() {
                        if looks_like_path(target) {
                            targets.push(target.clone());
                        }
                    }
                }
            }
            "dd" => {
                for arg in args {
                    if let Some(target) = arg.strip_prefix("of=") {
                        if !target.is_empty() && target != "/dev/null" {
                            targets.push(target.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    targets
}

fn shell_command_segments(tokens: &[ShellToken]) -> Vec<&[ShellToken]> {
    let mut segments = Vec::new();
    let mut start = 0;

    for (index, token) in tokens.iter().enumerate() {
        if !matches!(
            token,
            ShellToken::Pipe | ShellToken::And | ShellToken::Or | ShellToken::Semicolon
        ) {
            continue;
        }
        if start < index {
            segments.push(&tokens[start..index]);
        }
        start = index + 1;
    }
    if start < tokens.len() {
        segments.push(&tokens[start..]);
    }

    segments
}

fn simple_command_words(tokens: &[ShellToken]) -> Vec<String> {
    let mut words = Vec::new();
    let mut skip_next = false;

    for token in tokens {
        match token {
            ShellToken::Redirect { .. } | ShellToken::InputRedirect => {
                skip_next = true;
            }
            ShellToken::Word(word) => {
                if skip_next {
                    skip_next = false;
                } else {
                    words.push(word.clone());
                }
            }
            ShellToken::Pipe | ShellToken::And | ShellToken::Or | ShellToken::Semicolon => {}
        }
    }

    words
}

fn executable_name(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

fn find_command_index(words: &[String]) -> Option<usize> {
    let mut index = 0;
    while let Some(word) = words.get(index) {
        match executable_name(word) {
            "command" | "exec" | "builtin" => index += 1,
            "env" => {
                index += 1;
                while let Some(argument) = words.get(index) {
                    if argument.starts_with('-') || argument.contains('=') {
                        index += 1;
                    } else {
                        break;
                    }
                }
            }
            "sudo" if index == 0 => index += 1,
            _ => return Some(index),
        }
    }

    None
}

fn first_positional_arg(args: &[String]) -> Option<&str> {
    let mut options_ended = false;
    for arg in args {
        if !options_ended && arg == "--" {
            options_ended = true;
            continue;
        }
        if !options_ended && arg.starts_with('-') {
            continue;
        }
        return Some(arg);
    }
    None
}

fn positional_args(args: &[String]) -> Vec<&String> {
    let mut values = Vec::new();
    let mut options_ended = false;
    for arg in args {
        if !options_ended && arg == "--" {
            options_ended = true;
            continue;
        }
        if !options_ended && arg.starts_with('-') {
            continue;
        }
        values.push(arg);
    }
    values
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
