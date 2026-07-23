// dow/src/commands/
// ├── iterate.rs  -- dow iterate (Iteration Delivery: Validate → Archive → Commit + Tag → Bump)
//
// Related Docs:
// - [CLAUDE.md - Commands](../../../CLAUDE.md#Commands)
// - [dev-flow Specification](../../references/dev-flow-spec.md)

use crate::cli::IterateArgs;
use crate::core::{archive_db, doc_root, doc_validator, version, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct IterateOutput {
    released_version: String,
    tag: String,
    archive_db: String,
    archived_files: Vec<String>,
    next_version: String,
    next_phase: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pre_iterate: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    commit_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

#[derive(Clone, Debug)]
enum PreIterateStep {
    SyncVersion { path: String },
    Run { name: String, command: String },
}

pub fn run(args: IterateArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new("STATUS.yaml does not exist", 1));
    }

    let mode = yaml::get(&status_file, "mode")
        .map_err(|e| DowError::new(e.to_string(), 1))?
        .unwrap_or_else(|| "quick".to_string());

    let effective_mode = if mode.starts_with("audit/") {
        mode[6..].to_string()
    } else {
        mode.clone()
    };

    // 1. Validate: Task completion (skip in audit mode)
    if !mode.starts_with("audit/") {
        let (total, done) = count_tasks(&doc_root_path);
        if total > 0 && done < total {
            return Err(DowError::new(
                format!("Not all tasks completed ({}/{})", done, total),
                1,
            ));
        }
    }

    // 2. Validate: No open P0 issues
    let p0_open = count_p0_issues(&doc_root_path);
    if p0_open > 0 {
        return Err(DowError::new(
            format!("{} open P0 issue(s) found", p0_open),
            1,
        ));
    }

    // 2.5 Validate: All .dev-doc files are valid
    let validation_errors = doc_validator::validate_all(&doc_root_path);
    if !validation_errors.is_empty() {
        let msg = format!(
            "iterate pre-check failed: .dev-doc files contain format errors.\n{}",
            doc_validator::format_errors_human(&validation_errors)
        );
        return Err(DowError::new(msg, 1));
    }

    // 2.6 Check persistent docs sync (warn but don't block)
    let doc_warnings = check_persistent_docs_sync(&status_file);
    if !doc_warnings.is_empty() && args.confirm.is_none() {
        if human {
            println!("[dev-flow] Warning: The following persistent documents have not been updated since last iteration:");
            for w in &doc_warnings {
                println!("  - {}", w);
            }
            println!();
        }
    }

    // 3. Calculate version: -v controls the released version level, next is always +patch
    let current_version = version::read_current()?;
    let released_version = if args.bump == "patch" {
        current_version.clone()
    } else {
        version::bump_version_str(&current_version, &args.bump)?
    };
    let next_version = version::bump_version_str(&released_version, "patch")?;

    // 4. Calculate archive content
    let archive_base = archive_db::archive_base();
    let archive_db_path = format!("{}/archive.db", archive_base.to_string_lossy());
    let archived_files = list_archive_files(&doc_root_path);

    // --confirm mode: Execute after token verification
    if let Some(ref token) = args.confirm {
        let tokens = generate_tokens_with_window(&args);
        let bare_token = token.strip_prefix("ITR-").unwrap_or(token);

        let token_matches = tokens.iter().any(|t| {
            let bare_t = t.strip_prefix("ITR-").unwrap_or(t);
            bare_t == bare_token
        });

        if !token_matches {
            let hint = &tokens[0];
            return Err(DowError::new(
                format!("Confirmation failed: token mismatch. Expected {}", hint),
                1,
            ));
        }
    } else {
        // Trigger save_changelog before preview to ensure current session activity is recorded
        if let Err(e) = crate::hooks::save_changelog::run(false, false) {
            eprintln!("[dev-flow] save_changelog warning: {}", e.message);
        }

        // Generate token (preview mode)
        let token = generate_token_for_minute(0, &args);
        // Default: Output preview
        let mut commit_files = list_pending_changes(&args.files);
        // Filter out .dev-doc/ paths if gitignored
        if is_dev_doc_gitignored() {
            commit_files.retain(|f| !f.starts_with(".dev-doc/") && !f.starts_with(".dev-doc\\"));
        }
        let should_tag = args.bump != "patch" || args.tag;
        let changelog_entries = read_changelog_entries(&doc_root_path);
        let pre_iterate = describe_pre_iterate_steps()?;
        let result = IterateOutput {
            tag: if should_tag {
                format!("v{}", &released_version)
            } else {
                "no-tag".to_string()
            },
            released_version: released_version.clone(),
            archive_db: archive_db_path.clone(),
            archived_files: archived_files.clone(),
            next_version: next_version.clone(),
            next_phase: next_phase(&effective_mode, &mode),
            pre_iterate,
            commit_files,
            token: Some(token),
        };
        if human {
            print_human_preview(&result);
            print_changelog_summary(&changelog_entries);
        } else {
            let mut json_out = serde_json::to_value(&result).unwrap_or_default();
            json_out["changelog_entries"] = serde_json::json!(changelog_entries);
            json_out["changelog_hint"] = serde_json::json!(
                "Please check if CHANGELOG has any missing entries. If so, manually add them before confirmation."
            );
            if !doc_warnings.is_empty() {
                json_out["doc_sync_warnings"] = serde_json::json!(doc_warnings);
            }
            println!("{}", serde_json::to_string_pretty(&json_out).unwrap());
        }
        return Ok(0);
    }

    // 5. Read CHANGELOG (before archive, preserve commit body content)
    let changelog_entries = read_changelog_entries(&doc_root_path);

    // 5.1 Validate user-specified files first to avoid failure after archive due to bad path
    validate_git_add_inputs(&args.files)?;

    // 5.5 preIterate CI: Must execute before archive, commit, tag, bump; failure blocks entire iterate
    let pre_iterate = run_pre_iterate(&released_version, human)?;

    let mut commit_files = args.files.clone();
    for file in &pre_iterate {
        if !commit_files.contains(file) {
            commit_files.push(file.clone());
        }
    }
    validate_git_add_inputs(&commit_files)?;

    // 6. bump VERSION write first (archive version)
    version::write_current(&released_version)?;

    // 7. Execute archive (write to SQLite)
    let conn = archive_db::open_or_create(&archive_base)?;
    let released_at = chrono::Local::now().format("%Y-%m-%d").to_string();
    let cur_branch = doc_root::current_branch().unwrap_or_else(|| "main".to_string());
    // Whether a git tag will be created for this release (minor/major or --tag).
    // The archived `tag` field must match this so the archive reflects reality.
    let should_tag = args.bump != "patch" || args.tag;
    archive_db::insert_iteration(
        &conn,
        &archive_db::IterationRecord {
            version: released_version.clone(),
            topic: args.topic.clone(),
            commit_type: Some(args.r#type.clone()),
            branch: cur_branch,
            released_at,
            tag: if should_tag {
                format!("v{}", released_version)
            } else {
                "no-tag".to_string()
            },
        },
    )?;

    // Archive task files
    let task_dir = doc_root_path.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("done_task_") || name.starts_with("task_"))
            {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let tasks = archive_db::parse_task_file(&name, &content);
                    for task in &tasks {
                        archive_db::insert_task(&conn, &released_version, task)?;
                    }
                }
                if let Err(e) = fs::remove_file(entry.path()) {
                    eprintln!(
                        "[dev-flow] Warning: Failed to delete {} after archive: {}",
                        name, e
                    );
                }
            }
        }
    }

    // Archive closed issues
    let issue_dir = doc_root_path.join("issue");
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("closed_issue_") && name.ends_with(".md") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let issues = archive_db::parse_issue_file(&name, &content);
                    for issue in &issues {
                        archive_db::insert_issue(&conn, &released_version, issue)?;
                    }
                }
                if let Err(e) = fs::remove_file(entry.path()) {
                    eprintln!(
                        "[dev-flow] Warning: Failed to delete {} after archive: {}",
                        name, e
                    );
                }
            }
        }
    }

    // Archive documents (PRD/SPEC/TEST/BRAINSTORM)
    for doc_type in &["PRD", "SPEC", "TEST", "BRAINSTORM"] {
        let src = doc_root_path.join(format!("{}.md", doc_type));
        if src.exists() {
            if let Ok(content) = fs::read_to_string(&src) {
                archive_db::insert_doc(&conn, &released_version, doc_type, &content)?;
            }
            if let Err(e) = fs::remove_file(&src) {
                eprintln!(
                    "[dev-flow] Warning: Failed to delete {}.md after archive: {}",
                    doc_type, e
                );
            }
        }
    }

    // Archive CHANGELOG
    let changelog = doc_root_path.join("CHANGELOG.md");
    if changelog.exists() {
        if let Ok(content) = fs::read_to_string(&changelog) {
            let cl_entries = archive_db::parse_changelog(&content);
            for (order, (date, text)) in cl_entries.iter().enumerate() {
                archive_db::insert_changelog(
                    &conn,
                    &released_version,
                    date.as_deref(),
                    text,
                    order as i32,
                )?;
            }
        }
        fs::write(&changelog, "# Changelog\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 7.5 Clean up claim.lock (no longer needed after archive)
    let claim_lock = doc_root_path.join("claim.lock");
    if claim_lock.exists() {
        let _ = fs::remove_file(&claim_lock);
    }

    // 8. git commit + tag (include archive.db in commit)
    let commit_msg = format_commit_message(
        &released_version,
        &args.topic,
        &args.r#type,
        &changelog_entries,
    );
    // Only add archive_db_path if .dev-doc/ is not gitignored
    if !is_dev_doc_gitignored() {
        commit_files.push(archive_db_path.clone());
    }
    // Filter out all .dev-doc/ paths if gitignored
    let commit_files: Vec<String> = if is_dev_doc_gitignored() {
        commit_files.into_iter()
            .filter(|f| !f.starts_with(".dev-doc/") && !f.starts_with(".dev-doc\\"))
            .collect()
    } else {
        commit_files
    };
    git_commit(&commit_msg, &commit_files)?;

    // Only create git tag for minor/major or explicit --tag
    if should_tag {
        git_tag(&released_version)?;
    }

    // 9. bump VERSION to next_version + reset phase
    version::write_current(&next_version)?;
    let next_ph = next_phase(&effective_mode, &mode);

    if mode.starts_with("audit/") {
        yaml::set(&status_file, "mode", &effective_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    yaml::set(&status_file, "phase", &next_ph).map_err(|e| DowError::new(e.to_string(), 1))?;
    yaml::touch_updated(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;

    let tag_str = if should_tag {
        format!("v{}", released_version)
    } else {
        "no-tag".to_string()
    };
    let result = IterateOutput {
        released_version: released_version.clone(),
        tag: tag_str.clone(),
        archive_db: archive_db_path,
        archived_files,
        next_version: next_version,
        next_phase: next_ph,
        pre_iterate,
        commit_files: vec![],
        token: None,
    };

    if human {
        println!("[dev-flow] Iteration complete");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        let tag_display = if should_tag { " (tagged)" } else { "" };
        println!("Released version: v{}{}", released_version, tag_display);
        println!("New version: v{}", result.next_version);
        println!("Phase reset: {}", result.next_phase);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn count_tasks(doc_root: &Path) -> (u32, u32) {
    let task_dir = doc_root.join("task");
    let mut total = 0u32;
    let mut done = 0u32;

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (!name.starts_with("task_") && !name.starts_with("done_task_"))
                || !name.ends_with(".md")
            {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                total += content.lines().filter(|l| l.starts_with("- [")).count() as u32;
                done += content.lines().filter(|l| l.starts_with("- [x]")).count() as u32;
            }
        }
    }
    (total, done)
}

fn describe_pre_iterate_steps() -> Result<Vec<String>, DowError> {
    let steps = read_pre_iterate_steps()?;
    Ok(steps.iter().map(describe_pre_iterate_step).collect())
}

fn run_pre_iterate(version: &str, human: bool) -> Result<Vec<String>, DowError> {
    let steps = read_pre_iterate_steps()?;
    if steps.is_empty() {
        return Ok(Vec::new());
    }

    let snapshot = PreIterateSnapshot::capture();
    if human {
        println!("[dev-flow] Executing preIterate steps");
    }

    let mut changed_files = Vec::new();
    for step in steps {
        let result = match step {
            PreIterateStep::SyncVersion { path } => {
                if human {
                    println!("  - sync-version: {} -> {}", path, version);
                }
                let changed = sync_version_file(&path, version);
                if matches!(changed, Ok(true)) && !changed_files.contains(&path) {
                    changed_files.push(path);
                }
                changed.map(|_| ())
            }
            PreIterateStep::Run { name, command } => {
                if human {
                    println!("  - {}: {}", name, command);
                }
                run_shell_step(&name, &command)
            }
        };
        if let Err(err) = result {
            snapshot.restore().map_err(|rollback_err| {
                DowError::new(
                    format!(
                        "{}; preIterate rollback failed: {}",
                        err.message, rollback_err.message
                    ),
                    1,
                )
            })?;
            return Err(DowError::new(
                format!("{}; preIterate changes have been rolled back", err.message),
                err.exit_code,
            ));
        }
    }
    for file in git_worktree_paths() {
        if !snapshot.dirty_paths.contains(&file) && !changed_files.contains(&file) {
            changed_files.push(file);
        }
    }
    Ok(changed_files)
}

struct PreIterateSnapshot {
    files: BTreeMap<String, Option<Vec<u8>>>,
    dirty_paths: Vec<String>,
}

impl PreIterateSnapshot {
    fn capture() -> Self {
        let mut paths: BTreeSet<String> = git_tracked_paths().into_iter().collect();
        let dirty_paths = git_worktree_paths();
        paths.extend(dirty_paths.iter().cloned());

        let files = paths
            .into_iter()
            .map(|path| {
                let content = fs::read(&path).ok();
                (path, content)
            })
            .collect();

        Self { files, dirty_paths }
    }

    fn restore(&self) -> Result<(), DowError> {
        let known_paths: BTreeSet<&String> = self.files.keys().collect();

        for path in git_worktree_paths() {
            if !known_paths.contains(&path) && Path::new(&path).exists() {
                remove_path(Path::new(&path))?;
            }
        }

        for (path, content) in &self.files {
            let target = Path::new(path);
            match content {
                Some(bytes) => {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|e| {
                            DowError::new(
                                format!(
                                    "Failed to create rollback directory {}: {}",
                                    parent.display(),
                                    e
                                ),
                                1,
                            )
                        })?;
                    }
                    fs::write(target, bytes).map_err(|e| {
                        DowError::new(
                            format!("Failed to write rollback file {}: {}", target.display(), e),
                            1,
                        )
                    })?;
                }
                None => {
                    if target.exists() {
                        remove_path(target)?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn remove_path(path: &Path) -> Result<(), DowError> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| {
        DowError::new(
            format!("Failed to delete {} during rollback: {}", path.display(), e),
            1,
        )
    })
}

fn read_pre_iterate_steps() -> Result<Vec<PreIterateStep>, DowError> {
    let path = Path::new(".dev-doc/preIterate.ci");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("Failed to read .dev-doc/preIterate.ci: {}", e), 1))?;
    parse_pre_iterate_steps(&content)
}

fn parse_pre_iterate_steps(content: &str) -> Result<Vec<PreIterateStep>, DowError> {
    let mut steps = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "sync-version" {
            return Err(DowError::new(
                "preIterate sync-version must explicitly declare target file, e.g., `sync-version: dow/Cargo.toml`",
                1,
            ));
        }
        if let Some(path) = trimmed.strip_prefix("sync-version:") {
            let path = unquote(path.trim());
            if path.is_empty() {
                return Err(DowError::new(
                    "preIterate sync-version target cannot be empty",
                    1,
                ));
            }
            steps.push(PreIterateStep::SyncVersion { path });
            continue;
        }
        if let Some(command) = trimmed.strip_prefix("run:") {
            let command = unquote(command.trim());
            if command.is_empty() {
                return Err(DowError::new("preIterate run step cannot be empty", 1));
            }
            steps.push(PreIterateStep::Run {
                name: format!("run: {}", command),
                command,
            });
            continue;
        }
        return Err(DowError::new(
            format!("Unsupported preIterate step: {}", trimmed),
            1,
        ));
    }
    Ok(steps)
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn describe_pre_iterate_step(step: &PreIterateStep) -> String {
    match step {
        PreIterateStep::SyncVersion { path } => format!("sync-version: {}", path),
        PreIterateStep::Run { name, command } => format!("{}: {}", name, command),
    }
}

fn run_shell_step(name: &str, command: &str) -> Result<(), DowError> {
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    }
    .map_err(|e| {
        DowError::new(
            format!("preIterate step `{}` failed to start: {}", name, e),
            1,
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        return Err(DowError::new(
            format!("preIterate step `{}` failed: {}", name, detail),
            1,
        ));
    }
    Ok(())
}

fn sync_version_file(path: &str, version: &str) -> Result<bool, DowError> {
    let manifest = Path::new(path);
    if !manifest.exists() {
        return Err(DowError::new(
            format!("preIterate sync-version target does not exist: {}", path),
            1,
        ));
    }
    match manifest.file_name().and_then(|n| n.to_str()) {
        Some("Cargo.toml") => update_toml_version(manifest, version, &["package"]),
        Some("package.json") => update_package_json_version(manifest, version),
        Some("pyproject.toml") => {
            let project = update_toml_version(manifest, version, &["project"])?;
            let poetry = update_toml_version(manifest, version, &["tool", "poetry"])?;
            Ok(project || poetry)
        }
        _ => Err(DowError::new(
            format!("preIterate sync-version unsupported file: {}", path),
            1,
        )),
    }
}

fn update_package_json_version(path: &Path, version: &str) -> Result<bool, DowError> {
    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("Failed to read {}: {}", path.display(), e), 1))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| DowError::new(format!("Failed to parse {}: {}", path.display(), e), 1))?;
    let current = json.get("version").and_then(|v| v.as_str());
    if current == Some(version) {
        return Ok(false);
    }
    json["version"] = serde_json::Value::String(version.to_string());
    let output = serde_json::to_string_pretty(&json)
        .map_err(|e| DowError::new(format!("Failed to serialize {}: {}", path.display(), e), 1))?;
    fs::write(path, format!("{}\n", output))
        .map_err(|e| DowError::new(format!("Failed to write {}: {}", path.display(), e), 1))?;
    Ok(true)
}

fn update_toml_version(path: &Path, version: &str, section: &[&str]) -> Result<bool, DowError> {
    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("Failed to read {}: {}", path.display(), e), 1))?;
    let _: toml::Value = toml::from_str(&content)
        .map_err(|e| DowError::new(format!("Failed to parse {}: {}", path.display(), e), 1))?;

    let target_section: Vec<String> = section.iter().map(|key| key.to_string()).collect();
    let mut current_section: Vec<String> = Vec::new();
    let mut changed = false;
    let mut output = Vec::new();

    for line in content.lines() {
        let mut next_line = line.to_string();
        let trimmed = line.trim();
        if let Some(header) = parse_toml_section_header(trimmed) {
            current_section = header;
        } else if current_section == target_section {
            if let Some((updated, line_changed)) = replace_toml_version_line(line, version) {
                next_line = updated;
                changed |= line_changed;
            }
        }
        output.push(next_line);
    }

    if !changed {
        return Ok(false);
    }

    let mut output = output.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    fs::write(path, output)
        .map_err(|e| DowError::new(format!("Failed to write {}: {}", path.display(), e), 1))?;
    Ok(true)
}

fn parse_toml_section_header(trimmed: &str) -> Option<Vec<String>> {
    if let Some(rest) = trimmed.strip_prefix("[[") {
        let end = rest.find("]]")?;
        return Some(vec![format!("[[{}]]", &rest[..end])]);
    }
    if let Some(rest) = trimmed.strip_prefix('[') {
        let end = rest.find(']')?;
        return Some(
            rest[..end]
                .split('.')
                .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_string())
                .collect(),
        );
    }
    None
}

fn replace_toml_version_line(line: &str, version: &str) -> Option<(String, bool)> {
    let leading_len = line.len() - line.trim_start().len();
    let trimmed = &line[leading_len..];
    let rest = trimmed.strip_prefix("version")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-')
    {
        return None;
    }

    let equals = rest.find('=')?;
    let value_start = leading_len + "version".len() + equals + 1;
    let after_equals = &line[value_start..];
    let value_start = value_start + (after_equals.len() - after_equals.trim_start().len());
    let replacement = format!("\"{}\"", version);

    if let Some(quote) = line[value_start..]
        .chars()
        .next()
        .filter(|ch| *ch == '"' || *ch == '\'')
    {
        let value_end = find_toml_string_end(line, value_start, quote)?;
        let current = &line[value_start + quote.len_utf8()..value_end - quote.len_utf8()];
        if current == version {
            return Some((line.to_string(), false));
        }
        return Some((
            format!(
                "{}{}{}",
                &line[..value_start],
                replacement,
                &line[value_end..]
            ),
            true,
        ));
    }

    let comment_start = line[value_start..]
        .find('#')
        .map(|pos| value_start + pos)
        .unwrap_or(line.len());
    let value_end = line[..comment_start].trim_end().len();
    if line[value_start..value_end].trim() == version {
        return Some((line.to_string(), false));
    }
    Some((
        format!(
            "{}{}{}",
            &line[..value_start],
            replacement,
            &line[value_end..]
        ),
        true,
    ))
}

fn find_toml_string_end(line: &str, value_start: usize, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (offset, ch) in line[value_start + quote.len_utf8()..].char_indices() {
        if quote == '"' && escaped {
            escaped = false;
            continue;
        }
        if quote == '"' && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(value_start + quote.len_utf8() + offset + quote.len_utf8());
        }
    }
    None
}

fn git_worktree_paths() -> Vec<String> {
    let Ok(output) = Command::new("git").args(["status", "--porcelain"]).output() else {
        return Vec::new();
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let path = line[3..]
                .split(" -> ")
                .last()
                .unwrap_or("")
                .trim()
                .to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        })
        .collect()
}

fn git_tracked_paths() -> Vec<String> {
    let Ok(output) = Command::new("git").args(["ls-files", "-z"]).output() else {
        return Vec::new();
    };
    output
        .stdout
        .split(|b| *b == 0)
        .filter(|path| !path.is_empty())
        .filter_map(|path| String::from_utf8(path.to_vec()).ok())
        .collect()
}

fn count_p0_issues(doc_root: &Path) -> u32 {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return 0;
    }

    let mut p0_open = 0u32;
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("issue_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let mut in_open = false;
                for line in content.lines() {
                    if line.starts_with("- [ ]") {
                        in_open = true;
                    } else if line.starts_with("- [x]") {
                        in_open = false;
                    } else if in_open && line.contains("severity:") && line.contains("P0") {
                        p0_open += 1;
                        in_open = false;
                    }
                }
            }
        }
    }
    p0_open
}

fn next_phase(effective_mode: &str, _full_mode: &str) -> String {
    match effective_mode {
        "full" => "PRD",
        "quick" | "mvp" => "SPEC",
        "fast" => "TASK",
        _ => "DEV",
    }
    .to_string()
}

fn list_archive_files(doc_root: &Path) -> Vec<String> {
    let mut files = Vec::new();

    let task_dir = doc_root.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                files.push(name);
            }
        }
    }

    let issue_dir = doc_root.join("issue");
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("closed_issue_") && name.ends_with(".md") {
                files.push(name);
            }
        }
    }

    for doc in &[
        "PRD.md",
        "SPEC.md",
        "TEST.md",
        "BRAINSTORM.md",
        "CHANGELOG.md",
    ] {
        if doc_root.join(doc).exists() {
            files.push(doc.to_string());
        }
    }

    files
}

fn git_commit(message: &str, extra_files: &[String]) -> Result<(), DowError> {
    // Tracked file modifications/deletions
    run_git(["add", "-u"], "git add -u")?;
    // Additional specified new files/directories (skip paths deleted by archive)
    for f in extra_files {
        if !Path::new(f).exists() {
            continue;
        }
        run_git(["add", f.as_str()], &format!("git add {}", f))?;
    }

    let diff = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map_err(|e| DowError::new(format!("git diff --cached failed: {}", e), 1))?;

    if diff.status.success() {
        return Err(DowError::new(
            "git commit skipped: no staged changes. Please check iterate file list and git add output.",
            1,
        ));
    }

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DowError::new(format!("git commit failed: {}", stderr), 1));
    }
    Ok(())
}

fn validate_git_add_inputs(files: &[String]) -> Result<(), DowError> {
    for file in files {
        if file.trim().is_empty() {
            return Err(DowError::new("iterate --files contains empty path", 1));
        }
        if !Path::new(file).exists() {
            return Err(DowError::new(
                format!(
                    "iterate --files path does not exist, stopped before archive: {}",
                    file
                ),
                1,
            ));
        }
    }
    Ok(())
}

fn run_git<const N: usize>(args: [&str; N], label: &str) -> Result<(), DowError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| DowError::new(format!("{} failed to start: {}", label, e), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        return Err(DowError::new(format!("{} failed: {}", label, detail), 1));
    }
    Ok(())
}

fn is_dev_doc_gitignored() -> bool {
    Command::new("git")
        .args(["check-ignore", "-q", ".dev-doc/"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_tag(version: &str) -> Result<(), DowError> {
    let tag = format!("v{}", version);

    // Check if tag already exists
    let check = Command::new("git")
        .args(["tag", "-l", &tag])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !String::from_utf8_lossy(&check.stdout).trim().is_empty() {
        return Ok(()); // tag already exists
    }

    let output = Command::new("git")
        .args(["tag", "-a", &tag, "-m", &format!("Release {}", tag)])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DowError::new(format!("git tag failed: {}", stderr), 1));
    }
    Ok(())
}

fn read_changelog_entries(doc_root: &Path) -> Vec<String> {
    let changelog = doc_root.join("CHANGELOG.md");
    let mut entries = Vec::new();

    if let Ok(content) = fs::read_to_string(&changelog) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                entries.push(trimmed.to_string());
            }
        }
    }
    entries
}

fn format_commit_message(
    version: &str,
    topic: &str,
    commit_type: &str,
    changelog: &[String],
) -> String {
    let mut msg = format!("{}: v{} {}", commit_type, version, topic);
    if !changelog.is_empty() {
        msg.push_str("\n\n");
        for entry in changelog {
            msg.push_str(entry);
            msg.push('\n');
        }
    }
    msg
}

fn print_changelog_summary(entries: &[String]) {
    if entries.is_empty() {
        println!();
        println!("⚠ CHANGELOG is empty, please check if there are missing entries.");
        println!(
            "  If there are any omissions, manually add them to CHANGELOG.md before confirmation."
        );
    } else {
        println!();
        println!("CHANGELOG current entries ({} entries):", entries.len());
        for entry in entries {
            println!("  {}", entry);
        }
        println!();
        println!("Tip: Check if CHANGELOG has any omissions. If needed, edit CHANGELOG.md before confirmation.");
    }
}

fn print_human_preview(result: &IterateOutput) {
    println!("[dev-flow] Iteration Preview");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("Current version: v{}", result.released_version);
    println!("Archive database: {}", result.archive_db);
    println!("Archived files ({} items):", result.archived_files.len());
    for f in &result.archived_files {
        println!("  - {}", f);
    }
    println!();
    if !result.commit_files.is_empty() {
        println!("Commit files ({} items):", result.commit_files.len());
        for f in &result.commit_files {
            println!("  - {}", f);
        }
        println!();
    }
    if !result.pre_iterate.is_empty() {
        println!("preIterate steps ({} items):", result.pre_iterate.len());
        for step in &result.pre_iterate {
            println!("  - {}", step);
        }
        println!();
    }
    println!("Will execute:");
    println!("  - git commit + tag: v{}", result.released_version);
    if !result.pre_iterate.is_empty() {
        println!("  - preIterate: execute before git commit");
    }
    println!(
        "  - bump: v{} → v{}",
        result.released_version, result.next_version
    );
    println!("  - Phase reset: {}", result.next_phase);
    if let Some(ref t) = result.token {
        let bare = t.strip_prefix("ITR-").unwrap_or(t);
        println!();
        println!("Confirm: dow iterate --confirm {} ...", t);
        println!(
            "  (or: DOW_ITERATE_{}=1 dow iterate --confirm {} ...)",
            bare, t
        );
    }
}

fn generate_token_for_minute(offset: i64, args: &IterateArgs) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let now = chrono::Local::now() + chrono::Duration::minutes(offset);
    let minute_key = now.format("%Y-%m-%d-%H-%M").to_string();

    let mut hasher = DefaultHasher::new();
    cwd.hash(&mut hasher);
    minute_key.hash(&mut hasher);
    args.topic.hash(&mut hasher);
    args.r#type.hash(&mut hasher);
    args.bump.hash(&mut hasher);
    args.files.hash(&mut hasher);
    let hash = hasher.finish();
    format!("ITR-{}", &format!("{:016x}", hash)[..6])
}

// Return tokens for current minute + previous 4 minutes (5-minute validity window)
fn generate_tokens_with_window(args: &IterateArgs) -> Vec<String> {
    (0..=4)
        .map(|i| generate_token_for_minute(-i, args))
        .collect()
}

fn check_persistent_docs_sync(status_file: &Path) -> Vec<String> {
    let docs = yaml::get_list(status_file, "docs").unwrap_or_default();
    if docs.is_empty() {
        return Vec::new();
    }

    // Find the latest tag as --since reference
    let last_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let git_ref = match last_tag {
        Some(ref t) if !t.is_empty() => t.as_str(),
        _ => return Vec::new(),
    };

    // Verify ref is valid
    let ref_check = Command::new("git")
        .args(["rev-parse", "--verify", git_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !ref_check {
        return Vec::new();
    }

    let mut all_docs = docs;
    all_docs.push("README.md".to_string());

    let mut outdated = Vec::new();
    for doc in &all_docs {
        let changed = Command::new("git")
            .args(["log", &format!("{}..HEAD", git_ref), "--", doc])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        if !changed && Path::new(doc).exists() {
            outdated.push(doc.clone());
        }
    }
    outdated
}

fn list_pending_changes(extra_files: &[String]) -> Vec<String> {
    let mut changes = Vec::new();

    // Tracked file working directory changes (content that git add -u will commit)
    if let Ok(output) = Command::new("git").args(["diff", "--name-only"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.is_empty() {
                changes.push(line.to_string());
            }
        }
    }

    // Already staged changes
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.is_empty() && !changes.contains(&line.to_string()) {
                changes.push(line.to_string());
            }
        }
    }

    // Additional specified files
    for f in extra_files {
        if !changes.contains(f) {
            changes.push(f.clone());
        }
    }

    changes
}
