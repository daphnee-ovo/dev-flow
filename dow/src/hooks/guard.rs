// dow/src/hooks/
// ├── guard.rs  -- dow hooks guard（文件写入守护）
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../../CLAUDE.md#hooks)

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use serde_json;
use std::io::Read as IoRead;
use std::path::{Component, Path, PathBuf};

// ─── GuardPath: 规范化绝对路径 ───────────────────────────────────────────────

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

    /// 取相对于某目录的剩余路径
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

// ─── GuardContext: 预计算的项目路径 ──────────────────────────────────────────

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
        // .github/copilot 特殊处理
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

// ─── Hook 输出 ───────────────────────────────────────────────────────────────

fn deny(reason: &str, kiro_hook: bool) -> Result<i32, DowError> {
    if kiro_hook {
        // kiro-cli: exit code 2 + stderr = 阻止工具执行
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
        // kiro-cli: exit code 2 阻止（kiro 没有 ask 中间态，只有 allow/block）
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

// ─── 主入口 ──────────────────────────────────────────────────────────────────

pub fn run(file: String, kiro_hook: bool) -> Result<i32, DowError> {
    let targets = resolve_targets(&file);
    if targets.is_empty() {
        return Ok(0);
    }

    let project_root = std::env::current_dir().unwrap_or_default();
    let ctx = GuardContext::new(project_root);

    for raw_target in &targets {
        let path = GuardPath::new(raw_target, &ctx.root);

        // 1. 项目边界检查
        if !path.is_under(&ctx.root) {
            if is_dangerous_system_path(&path) {
                return deny(&format!("[dev-flow] 禁止写入系统敏感路径：{}", raw_target), kiro_hook);
            }
            return ask(&format!(
                "[dev-flow] 写入目标在项目外：{}。请确认是否允许。",
                raw_target
            ), kiro_hook);
        }

        // 2. VERSION 保护
        if path.is_exact(&ctx.version_file) {
            return deny(
                "[dev-flow] 禁止直接修改 VERSION 文件。请使用 `dow version --set X.Y.Z` 或 `dow version --bump minor`。",
                kiro_hook,
            );
        }

        // 3. STATUS.yaml 保护
        if path.file_name() == Some("STATUS.yaml") && path.is_under(&ctx.devdoc_dir) {
            return deny(
                "[dev-flow] 禁止直接创建或修改 STATUS.yaml。请使用 `dow status --phase/--mode/--name` 或 `dow init`。",
                kiro_hook,
            );
        }

        // 4. .dev-doc 文件创建保护
        if let Some(reason) = check_devdoc_direct_create(&path, &ctx) {
            return deny(&reason, kiro_hook);
        }

        // 5. 跨分支写入保护
        if let Some(reason) = check_cross_branch(&path, &ctx) {
            return deny(&reason, kiro_hook);
        }

        // 6. 阶段性写入控制
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

// ─── 检查函数 ────────────────────────────────────────────────────────────────

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

    // 直接在 .dev-doc/ 下的文件（如 archive.db）
    if !rel_str.contains('/') && !rel_str.contains('\\') {
        return None;
    }

    let current = doc_root::current_branch()?;
    let current_branch_dir = ctx.devdoc_dir.join(&current);

    // 在当前分支目录下 → 允许
    if path.is_under(&current_branch_dir) {
        return None;
    }

    // 逐级检查是否属于其他已知分支目录
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
                "[dev-flow] BLOCKED: 当前分支为 `{}`，禁止写入其他分支的文档目录：{}\n→ 请确认你已切换到正确的分支，或使用 `git checkout {}` 切换。",
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

    // DEV/TEST 阶段
    if phase == "DEV" || phase == "TEST" {
        if mode.starts_with("audit/") {
            return None;
        }
        // 白名单
        if path.is_under(&ctx.devdoc_dir)
            || path.is_under(&ctx.tmp_dir)
            || path.is_under(&ctx.docs_dir)
            || ctx.is_ai_config(path)
        {
            return None;
        }
        // DEV 无活跃 claim → 区分原因
        if phase == "DEV" && !has_active_claim(&ctx.doc_root_path()) {
            let doc_root = ctx.doc_root_path();
            let has_undone = has_pending_work(&doc_root);
            if has_undone {
                return Some(PhaseDecision::Deny(format!(
                    "[dev-flow] DEV 阶段有未完成的 task/issue 但未 claim，不允许写入 {}。请先执行：\n\
                    → `dow claim <TASK_ID>` 认领要开发的任务",
                    path.display()
                )));
            } else {
                return Some(PhaseDecision::Deny(format!(
                    "[dev-flow] DEV 阶段所有 task 已完成且无 open issue，不允许写入 {}。请选择：\n\
                    → /task 创建新任务\n\
                    → /issue 创建 issue\n\
                    → /test 进入测试阶段",
                    path.display()
                )));
            }
        }
        return None;
    }

    // 非 DEV/TEST 阶段（PRD/SPEC/TASK 等）

    // 白名单 1: tmp（代码文件需确认）
    if path.is_under(&ctx.tmp_dir) {
        if is_code_file(path) {
            return Some(PhaseDecision::Ask(format!(
                "[dev-flow] 当前阶段为 {}，tmp/ 下写入代码文件：{}。确认是探索性 demo 吗？",
                phase,
                path.display()
            )));
        }
        return None;
    }

    // 白名单 2: AI 配置
    if ctx.is_ai_config(path) {
        return None;
    }

    // 白名单 3: docs（禁止代码文件）
    if path.is_under(&ctx.docs_dir) {
        if is_code_file(path) {
            return Some(PhaseDecision::Deny(format!(
                "[dev-flow] 当前阶段为 {}，docs/ 下不允许写入代码文件：{}。代码文件请在 DEV 阶段创建。",
                phase, path.display()
            )));
        }
        return None;
    }

    // 白名单 4: .dev-doc 工作流文件
    if path.is_under(&ctx.devdoc_dir) {
        if path.exists() || path.is_dir() {
            return None;
        }
        if is_valid_devdoc_file(path, ctx) {
            return None;
        }
        return Some(PhaseDecision::Deny(format!(
            "[dev-flow] .dev-doc/ 下不允许创建非工作流文件：{}。合法文件：PRD.md、SPEC.md、TEST.md、BRAINSTORM.md、CHANGELOG.md、task/task_*.md、issue/issue_*.md、STATUS.yaml",
            path.display()
        )));
    }

    // 其余 → deny
    Some(PhaseDecision::Deny(format!(
        "[dev-flow] 当前阶段为 {}，只允许写入 .dev-doc/、docs/ 和 tmp/。要写入 {} 请先完成规划并进入 DEV 阶段：创建任务（/task）或创建 issue（/issue）后即可进入 DEV。（探索性代码、demo 可放 tmp/ 下）",
        phase, path.display()
    )))
}

fn check_devdoc_direct_create(path: &GuardPath, ctx: &GuardContext) -> Option<String> {
    if !path.is_under(&ctx.devdoc_dir) {
        return None;
    }

    // 需要 branch 目录下的相对路径
    let branch_dir = ctx.current_branch_dir()?;
    let rel = path.relative_to(&branch_dir)?;
    let rel_str = rel.to_string_lossy().to_string();

    let protected_singles = [
        ("PRD.md", "prd"),
        ("SPEC.md", "spec"),
        ("TEST.md", "test"),
        ("BRAINSTORM.md", "brainstorm"),
        ("CHANGELOG.md", "changelog"),
    ];

    for (filename, doc_type) in &protected_singles {
        if rel_str == *filename {
            if !path.exists() {
                return Some(format!(
                    "[dev-flow] BLOCKED: 禁止手动创建 {}，请使用 `dow doc {}`",
                    path.display(),
                    doc_type
                ));
            }
            return None;
        }
    }

    // task/ 和 issue/ 下的新文件
    if (rel_str.starts_with("task/task_") || rel_str.starts_with("issue/issue_"))
        && rel_str.ends_with(".md")
        && is_standard_doc_filename(&rel_str)
    {
        if !path.exists() {
            let doc_type = if rel_str.starts_with("task/") {
                "task"
            } else {
                "issue"
            };
            return Some(format!(
                "[dev-flow] BLOCKED: 禁止手动创建 {}，请使用 `dow doc {} [-n N]`",
                path.display(),
                doc_type
            ));
        }
    }

    None
}

// ─── 辅助函数 ────────────────────────────────────────────────────────────────

fn has_active_claim(doc_root: &Path) -> bool {
    !crate::core::claim::get_active_claims(doc_root).is_empty()
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

fn is_standard_doc_filename(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() != 2 {
        return false;
    }
    let filename = parts[1];
    filename.chars().filter(|c| *c == '-').count() >= 2
        && filename.contains(|c: char| c.is_ascii_digit())
}

// ─── 输入解析（不变） ────────────────────────────────────────────────────────

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

    // 重定向写入：> file、>> file
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

    // tee 写入
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

    // cp/mv 目标
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
