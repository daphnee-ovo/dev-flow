// dow/src/commands/
// ├── lint.rs  -- dow lint (merged check + validate + fix)
//
// Unified linting command that merges logic from:
// - check.rs: changelog, task completion, issue status, time sync, phase files, spec AC, task nums
// - validate.rs: directory structure, file naming, gitignore
// - doc_validator::validate_all(): frontmatter, field validation, sequence checks
// - fix.rs: auto-fix logic (--fix mode)
//
// Related Docs:
// - [ISSUE Specification](../../../references/.dev-doc/ISSUE.md)
// - [TASK Specification](../../../references/.dev-doc/TASK-FILE.md)

use crate::cli::LintArgs;
use crate::core::{doc_root, doc_validator, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
struct LintOutput {
    pass: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    ok: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fixed: Vec<String>,
}

pub fn run(args: LintArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new("STATUS.yaml not found — run `dow init` first", 1));
    }

    let map = yaml::read(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;
    let phase = map.get("phase").cloned().unwrap_or_default();
    let mode = map.get("mode").cloned().unwrap_or_default();

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut ok: Vec<String> = Vec::new();
    let mut fixed: Vec<String> = Vec::new();

    // ══════════════════════════════════════════════════════════════════════
    // Phase 1: check.rs logic (document sync / consistency checks)
    // ══════════════════════════════════════════════════════════════════════

    check_changelog(&doc_root_path, &mut warnings, &mut ok);
    let (total_tasks, done_tasks) = check_tasks(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);
    check_issues(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);
    check_time_sync(&map, &mut warnings, &mut ok);
    check_phase_files(&doc_root_path, &phase, &mut warnings);
    check_spec_ac(&doc_root_path, &mode, &mut errors, &mut warnings);
    check_test_report(&doc_root_path, total_tasks, done_tasks, &mut warnings);
    check_task_nums(&doc_root_path, &mut errors);

    // ══════════════════════════════════════════════════════════════════════
    // Phase 2: validate.rs logic (structural checks)
    // ══════════════════════════════════════════════════════════════════════

    check_directories(&doc_root_path, &mut fixed);
    check_gitignore(&mut fixed);
    check_stale_root_files(&doc_root_path, &mut warnings);

    // ══════════════════════════════════════════════════════════════════════
    // Phase 3: doc_validator::validate_all() (content validation)
    // ══════════════════════════════════════════════════════════════════════

    let validation_errors = doc_validator::validate_all(&doc_root_path);
    for ve in &validation_errors {
        let prefix = if ve.fixable { "[fixable] " } else { "" };
        let msg = format!("{}{}: {}", prefix, ve.file, ve.message);
        errors.push(msg);
    }

    // ══════════════════════════════════════════════════════════════════════
    // Phase 4: --fix mode (auto-fix from fix.rs logic)
    // ══════════════════════════════════════════════════════════════════════

    if args.fix {
        let (fix_fixed, fix_unfixable) = run_auto_fix(&doc_root_path);
        fixed.extend(fix_fixed);

        // After fix: reset errors and re-validate from scratch
        errors.clear();
        warnings.clear();
        ok.clear();

        // Re-run all checks on the now-fixed state
        check_changelog(&doc_root_path, &mut warnings, &mut ok);
        check_tasks(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);
        check_issues(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);
        check_time_sync(&map, &mut warnings, &mut ok);
        check_phase_files(&doc_root_path, &phase, &mut warnings);
        check_spec_ac(&doc_root_path, &mode, &mut errors, &mut warnings);
        check_task_nums(&doc_root_path, &mut errors);

        let post_fix_errors = doc_validator::validate_all(&doc_root_path);
        for ve in &post_fix_errors {
            errors.push(format!("{}: {}", ve.file, ve.message));
        }

        // Add unfixable items
        for u in fix_unfixable {
            if !errors.contains(&u) {
                errors.push(u);
            }
        }
    }

    let pass = errors.is_empty();

    // Silent principle: if no issues and nothing fixed, exit 0 with no output
    if pass && warnings.is_empty() && fixed.is_empty() {
        return Ok(0);
    }

    let result = LintOutput { pass, errors, warnings, ok, fixed };

    if human {
        print_human(&result, &phase);
    } else {
        output::print_json(&result);
    }

    Ok(if pass { 0 } else { 1 })
}

// ══════════════════════════════════════════════════════════════════════════════
// Check logic (from check.rs)
// ══════════════════════════════════════════════════════════════════════════════

fn check_changelog(doc_root: &Path, warnings: &mut Vec<String>, ok: &mut Vec<String>) {
    let changelog = doc_root.join("CHANGELOG.md");
    if changelog.exists() {
        if fs::metadata(&changelog).map(|m| m.len() > 0).unwrap_or(false) {
            ok.push("CHANGELOG.md exists and is not empty".to_string());
        } else {
            warnings.push("CHANGELOG.md is empty".to_string());
        }
    } else {
        warnings.push("CHANGELOG.md not found".to_string());
    }
}

fn check_tasks(
    doc_root: &Path,
    phase: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    _ok: &mut Vec<String>,
) -> (usize, usize) {
    let task_dir = doc_root.join("task");
    let mut total = 0usize;
    let mut done = 0usize;

    if !task_dir.is_dir() {
        if matches!(phase, "DEV" | "TEST" | "DONE") {
            warnings.push(format!("Phase is {} but task/ directory has no tasks", phase));
        }
        return (0, 0);
    }

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if !name.starts_with("task_") && !name.starts_with("done_task_") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                total += content.lines().filter(|l| l.starts_with("- [")).count();
                done += content.lines().filter(|l| l.starts_with("- [x]")).count();
            }
        }
    }

    if phase == "DEV" && total == 0 {
        warnings.push("Phase is DEV but task/ directory has no task files".to_string());
    }
    if total > 0 && done == total && phase == "DEV" {
        warnings.push("All tasks completed but phase is still DEV, consider running /test".to_string());
    }

    let _ = errors; // nums check in separate function

    (total, done)
}

fn check_task_nums(doc_root: &Path, errors: &mut Vec<String>) {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let actual = content.lines().filter(|l| l.starts_with("- [")).count();
                for line in content.lines() {
                    if line.starts_with("nums:") {
                        if let Some(val) = line.strip_prefix("nums:") {
                            if let Ok(declared) = val.trim().parse::<usize>() {
                                if declared != actual {
                                    errors.push(format!(
                                        "task_nums_mismatch: {} declares nums={}, actual task count={}",
                                        name, declared, actual
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn check_issues(
    doc_root: &Path,
    phase: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return;
    }

    let mut open_issues = 0usize;
    let mut open_p0 = 0usize;

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
                        open_issues += 1;
                    } else if line.starts_with("- [x]") {
                        in_open = false;
                    } else if in_open && line.contains("severity:") && line.contains("P0") {
                        open_p0 += 1;
                        in_open = false;
                    }
                }
            }
        }
    }

    if open_p0 > 0 {
        errors.push(format!("open_p0_issue: {} unclosed P0 issues found", open_p0));
    }
    if open_issues > 0 && phase == "DONE" {
        warnings.push(format!("Phase is DONE but {} unclosed issues remain", open_issues));
    }
    if open_issues == 0 && issue_dir.exists() {
        ok.push("All issues are closed".to_string());
    }
}

fn check_time_sync(
    map: &BTreeMap<String, String>,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let updated = map.get("updated").cloned().unwrap_or_default();
    let status_date = updated.split(' ').next().unwrap_or("");

    let commit_date = Command::new("git")
        .args(["log", "-1", "--format=%ai"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .split(' ')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            }
        });

    if let Some(commit_date) = commit_date {
        if !commit_date.is_empty() && !status_date.is_empty() && commit_date.as_str() > status_date {
            warnings.push(format!(
                "Latest code commit ({}) is later than STATUS update ({}), documentation may be out of sync",
                commit_date, status_date
            ));
        } else {
            ok.push("STATUS update time is in sync with code".to_string());
        }
    }
}

fn check_phase_files(doc_root: &Path, phase: &str, warnings: &mut Vec<String>) {
    if matches!(phase, "SPEC" | "TASK" | "DEV" | "TEST" | "DONE") {
        if !doc_root.join("SPEC.md").exists() {
            warnings.push(format!("Phase is {} but SPEC.md is missing", phase));
        }
    }
}

fn check_spec_ac(doc_root: &Path, mode: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let spec_file = doc_root.join("SPEC.md");
    if !spec_file.exists() {
        return;
    }
    if let Ok(content) = fs::read_to_string(&spec_file) {
        let has_ac = content.contains("SPEC-AC-")
            || content.contains("## Acceptance")
            || content.contains("## 5. Acceptance Criteria")
            || content.contains("Acceptance");
        if !has_ac {
            match mode {
                "full" | "quick" => {
                    errors.push(format!(
                        "spec_missing_ac: {} mode SPEC missing testable acceptance criteria",
                        mode
                    ));
                }
                _ => {
                    warnings.push(
                        "SPEC lacks explicit acceptance criteria, consider adding them".to_string(),
                    );
                }
            }
        }
    }
}

fn check_test_report(doc_root: &Path, total: usize, done: usize, warnings: &mut Vec<String>) {
    if total > 0 && done == total {
        let test_file = doc_root.join("TEST.md");
        if !test_file.exists() {
            warnings.push("All tasks completed but TEST.md is missing".to_string());
        } else if let Ok(content) = fs::read_to_string(&test_file) {
            let has_failure = content.contains("FAILED SUITES:")
                || content.contains("FAIL: ")
                || content.contains("FAILED:")
                || content.contains("NOT PASSED:");
            if has_failure {
                warnings.push("TEST.md reports failed tests".to_string());
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Validate logic (from validate.rs)
// ══════════════════════════════════════════════════════════════════════════════

fn check_directories(doc_root: &Path, fixed: &mut Vec<String>) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp"
    } else {
        "tmp"
    };

    let dirs = [
        doc_root.join("issue"),
        doc_root.join("task"),
        PathBuf::from("tests"),
        PathBuf::from(project_temp),
    ];

    for dir in &dirs {
        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!(
                    "[dow] warning: failed to create directory ({}): {}",
                    dir.display(),
                    e
                );
            } else {
                fixed.push(format!("created_dir:{}", dir.display()));
            }
        }
    }
}

fn check_gitignore(fixed: &mut Vec<String>) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp/"
    } else {
        "tmp/"
    };
    let claim_lock_entry = ".dev-doc/**/claim.lock";

    let required_entries: &[&str] = &[project_temp, claim_lock_entry];

    if Path::new(".gitignore").exists() {
        let content = fs::read_to_string(".gitignore").unwrap_or_default();
        let mut new_content = content.clone();

        for &entry in required_entries {
            if !new_content.lines().any(|l| l.trim() == entry) {
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(entry);
                new_content.push('\n');
            }
        }

        if new_content != content {
            if let Err(e) = fs::write(".gitignore", &new_content) {
                eprintln!("[dow] warning: failed to update .gitignore: {}", e);
            } else {
                fixed.push("gitignore_updated".to_string());
            }
        }
    } else {
        let content = required_entries.join("\n") + "\n";
        if let Err(e) = fs::write(".gitignore", &content) {
            eprintln!("[dow] warning: failed to create .gitignore: {}", e);
        } else {
            fixed.push("gitignore_created".to_string());
        }
    }
}

fn check_stale_root_files(doc_root: &Path, warnings: &mut Vec<String>) {
    let base_path = Path::new(crate::core::DOC_DIR);
    if doc_root == base_path {
        return;
    }

    for subdir in &["issue", "task"] {
        let root_dir = base_path.join(subdir);
        if !root_dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&root_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                warnings.push(format!(
                    "stale file in .dev-doc root: {}/{} (should be in {})",
                    subdir,
                    name,
                    doc_root.display()
                ));
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Fix logic (from fix.rs)
// ══════════════════════════════════════════════════════════════════════════════

/// Run auto-fix on issue and task files, returns (fixed_items, unfixable_items)
fn run_auto_fix(doc_root: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();

    // Fix issue files
    let issue_dir = doc_root.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("issue_") || name.starts_with("closed_issue_"))
                {
                    let (f, u) = fix_issue_file(&entry.path());
                    fixed.extend(f);
                    unfixable.extend(u);
                }
            }
        }
    }

    // Fix task files
    let task_dir = doc_root.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("task_") || name.starts_with("done_task_"))
                {
                    let (f, u) = fix_task_file(&entry.path());
                    fixed.extend(f);
                    unfixable.extend(u);
                }
            }
        }
    }

    // Fix issue filenames missing source: extract from frontmatter and rename
    fix_issue_filename_source(&issue_dir, &mut fixed);

    // Fix issue state consistency: add closed_ prefix to fully checked issue files
    fix_issue_rename(&issue_dir, &mut fixed);

    // Fix task state consistency: add done_ prefix to fully checked task files
    fix_task_rename(&task_dir, &mut fixed);

    // Fix issue global sequence conflicts: renumber by file date
    fix_issue_renumber(&issue_dir, &mut fixed);

    (fixed, unfixable)
}

/// Fix a single issue file, returns (fixed, unfixable)
fn fix_issue_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_issue_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                let source = extract_source_from_filename(&filename).unwrap_or("other");
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\nsource: {}\nnums: {}\n---\n\n", source, item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}: added frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("source") {
                    let source = extract_source_from_filename(&filename).unwrap_or("other");
                    new_content = insert_fm_field(&new_content, "source", source);
                    needs_write = true;
                    fixed.push(format!("{}: added source field", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}: added nums field", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}: {}", filename, error.message));
            }
        }
    }

    if needs_write {
        if let Err(e) = fs::write(path, &new_content) {
            eprintln!("[dow lint] warning: failed to write {}: {}", filename, e);
            unfixable.push(format!("{}: write failed - {}", filename, e));
        }
    }

    (fixed, unfixable)
}

/// Fix a single task file, returns (fixed, unfixable)
fn fix_task_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_task_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\ntitle: TASK - \nnums: {}\n---\n\n", item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}: added frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("title") {
                    new_content = insert_fm_field(&new_content, "title", "TASK - ");
                    needs_write = true;
                    fixed.push(format!("{}: added title field", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}: added nums field", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}: {}", filename, error.message));
            }
        }
    }

    if needs_write {
        if let Err(e) = fs::write(path, &new_content) {
            eprintln!("[dow lint] warning: failed to write {}: {}", filename, e);
            unfixable.push(format!("{}: write failed - {}", filename, e));
        }
    }

    (fixed, unfixable)
}

/// Rename issue files with all items checked to closed_ prefix
fn fix_issue_filename_source(issue_dir: &Path, fixed: &mut Vec<String>) {
    if !issue_dir.is_dir() {
        return;
    }
    let entries: Vec<_> = match fs::read_dir(issue_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let (prefix, rest) = if name.starts_with("closed_issue_") {
            ("closed_issue_", &name["closed_issue_".len()..name.len() - 3])
        } else if name.starts_with("issue_") {
            ("issue_", &name["issue_".len()..name.len() - 3])
        } else {
            continue;
        };

        // Check if source is missing: rest starts with date (YYYY-MM-DD) not a source word
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.is_empty() {
            continue;
        }
        // If first part contains '-' it's likely a date, meaning source is missing
        if !parts[0].contains('-') {
            continue;
        }

        // Source is missing — extract from file frontmatter
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let source_owned = extract_source_from_frontmatter(&content);
        let source = source_owned.as_deref().unwrap_or("other");
        let new_name = format!("{}{}_{}_{}.md", prefix, source, parts[0], parts.get(1).unwrap_or(&"1"));
        if new_name == name {
            continue;
        }
        let new_path = issue_dir.join(&new_name);
        if new_path.exists() {
            continue;
        }
        if let Err(e) = fs::rename(entry.path(), &new_path) {
            eprintln!("[dow lint] warning: failed to rename {} -> {}: {}", name, new_name, e);
        } else {
            fixed.push(format!("{}: renamed to {} (added source)", name, new_name));
        }
    }
}

fn extract_source_from_frontmatter(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in content.lines() {
        if line.trim() == "---" {
            if in_frontmatter {
                return None;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if let Some(rest) = line.strip_prefix("source:") {
                let val = rest.trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn fix_issue_rename(issue_dir: &Path, fixed: &mut Vec<String>) {
    if !issue_dir.is_dir() {
        return;
    }
    let entries: Vec<_> = match fs::read_dir(issue_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("issue_") || !name.ends_with(".md") {
            continue;
        }
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let total = content.lines().filter(|l| l.starts_with("- [")).count();
        let done = content.lines().filter(|l| l.starts_with("- [x]")).count();
        if total > 0 && total == done {
            let new_name = format!("closed_{}", name);
            let new_path = issue_dir.join(&new_name);
            if !new_path.exists() {
                if let Err(e) = fs::rename(entry.path(), &new_path) {
                    eprintln!(
                        "[dow lint] warning: failed to rename {} -> {}: {}",
                        name, new_name, e
                    );
                } else {
                    fixed.push(format!("{}: renamed to {}", name, new_name));
                }
            }
        }
    }
}

/// Rename task files with all items checked to done_ prefix
fn fix_task_rename(task_dir: &Path, fixed: &mut Vec<String>) {
    if !task_dir.is_dir() {
        return;
    }
    let entries: Vec<_> = match fs::read_dir(task_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("task_") || !name.ends_with(".md") {
            continue;
        }
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let total = content.lines().filter(|l| l.starts_with("- [")).count();
        let done = content.lines().filter(|l| l.starts_with("- [x]")).count();
        if total > 0 && total == done {
            let new_name = format!("done_{}", name);
            let new_path = task_dir.join(&new_name);
            if !new_path.exists() {
                if let Err(e) = fs::rename(entry.path(), &new_path) {
                    eprintln!(
                        "[dow lint] warning: failed to rename {} -> {}: {}",
                        name, new_name, e
                    );
                } else {
                    fixed.push(format!("{}: renamed to {}", name, new_name));
                }
            }
        }
    }
}

/// Fix issue global sequence conflicts: renumber by file date
fn fix_issue_renumber(issue_dir: &Path, fixed: &mut Vec<String>) {
    if !issue_dir.is_dir() {
        return;
    }

    struct IssueItem {
        file_path: PathBuf,
        file_date: String,
        file_seq: u32,
        line_idx: usize,
        current_num: u32,
    }

    let mut items: Vec<IssueItem> = Vec::new();
    let mut seen_nums: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut has_conflict = false;

    let entries: Vec<_> = match fs::read_dir(issue_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        if !name.starts_with("issue_") && !name.starts_with("closed_issue_") {
            continue;
        }

        let (date, seq) = match parse_issue_file_date_seq(&name) {
            Some(v) => v,
            None => continue,
        };

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_idx, line) in content.lines().enumerate() {
            if !line.starts_with("- [") {
                continue;
            }
            let title = line[5..].trim();
            if let Some(num) = extract_issue_num(title) {
                if !seen_nums.insert(num) {
                    has_conflict = true;
                }
                items.push(IssueItem {
                    file_path: entry.path(),
                    file_date: date.clone(),
                    file_seq: seq,
                    line_idx,
                    current_num: num,
                });
            }
        }
    }

    if !has_conflict {
        if !items.is_empty() {
            let mut nums: Vec<u32> = items.iter().map(|i| i.current_num).collect();
            nums.sort();
            let is_sequential = nums.iter().enumerate().all(|(idx, &n)| n == (idx as u32 + 1));
            if is_sequential {
                return;
            }
        } else {
            return;
        }
    }

    // Sort by file date, file sequence, and line index
    items.sort_by(|a, b| {
        a.file_date
            .cmp(&b.file_date)
            .then(a.file_seq.cmp(&b.file_seq))
            .then(a.line_idx.cmp(&b.line_idx))
    });

    // Assign new sequence numbers
    let mut renames: std::collections::HashMap<PathBuf, Vec<(usize, u32, u32)>> =
        std::collections::HashMap::new();
    for (new_idx, item) in items.iter().enumerate() {
        let new_num = (new_idx + 1) as u32;
        if new_num != item.current_num {
            renames
                .entry(item.file_path.clone())
                .or_default()
                .push((item.line_idx, item.current_num, new_num));
        }
    }

    if renames.is_empty() {
        return;
    }

    // Apply replacements
    let mut total_fixed = 0u32;
    for (file_path, changes) in &renames {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        for &(line_idx, old_num, new_num) in changes {
            if line_idx < lines.len() {
                let old_id = format!("ISSUE-I{:03}", old_num);
                let new_id = format!("ISSUE-I{:03}", new_num);
                lines[line_idx] = lines[line_idx].replace(&old_id, &new_id);
            }
        }
        let new_content = lines.join("\n");
        let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };
        if let Err(e) = fs::write(file_path, &final_content) {
            eprintln!(
                "[dow lint] warning: failed to write {}: {}",
                file_path.display(),
                e
            );
        } else {
            total_fixed += changes.len() as u32;
        }
    }

    if total_fixed > 0 {
        fixed.push(format!(
            "issue global sequence renumbering: fixed {} items",
            total_fixed
        ));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Utility functions
// ══════════════════════════════════════════════════════════════════════════════

fn extract_source_from_filename(filename: &str) -> Option<&str> {
    let stem = filename.strip_suffix(".md")?;
    let rest = if stem.starts_with("closed_issue_") {
        &stem["closed_issue_".len()..]
    } else if stem.starts_with("issue_") {
        &stem["issue_".len()..]
    } else {
        return None;
    };
    // source_YYYY-MM-DD_seq
    rest.split('_').next()
}

fn insert_fm_field(content: &str, key: &str, value: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    if let Some(end_idx) = content[3..].find("---") {
        let fm_end = 3 + end_idx;
        let mut fm = content[3..fm_end].to_string();
        fm.push_str(&format!("{}: {}\n", key, value));
        format!("---{}---{}", fm, &content[fm_end + 3..])
    } else {
        content.to_string()
    }
}

fn parse_issue_file_date_seq(filename: &str) -> Option<(String, u32)> {
    let stem = filename.strip_suffix(".md")?;
    let rest = if stem.starts_with("closed_issue_") {
        &stem["closed_issue_".len()..]
    } else if stem.starts_with("issue_") {
        &stem["issue_".len()..]
    } else {
        return None;
    };
    // Try format: "source_YYYY-MM-DD_seq" (4 parts with date having dashes)
    // or fallback: "YYYY-MM-DD_seq" (missing source)
    let parts: Vec<&str> = rest.split('_').collect();

    // Detect date pattern (YYYY-MM-DD = 3 parts joined by -)
    // Full format: source_YYYY_MM_DD_seq → split gives [source, YYYY, MM, DD, seq] (won't work)
    // Actually dates use - not _: source_2026-06-25_seq → split('_') gives [source, 2026-06-25, seq]
    // Missing source: 2026-06-25_seq → split('_') gives [2026-06-25, seq]

    if parts.len() >= 3 && parts[1].contains('-') {
        // Normal: source_date_seq
        let date = parts[1].to_string();
        let seq = parts[2].parse::<u32>().unwrap_or(0);
        Some((date, seq))
    } else if parts.len() >= 2 && parts[0].contains('-') {
        // Missing source: date_seq
        let date = parts[0].to_string();
        let seq = parts[1].parse::<u32>().unwrap_or(0);
        Some((date, seq))
    } else {
        None
    }
}

fn extract_issue_num(title: &str) -> Option<u32> {
    let prefix = "ISSUE-I";
    if !title.starts_with(prefix) {
        return None;
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

// ══════════════════════════════════════════════════════════════════════════════
// Human output
// ══════════════════════════════════════════════════════════════════════════════

fn print_human(result: &LintOutput, phase: &str) {
    println!("[dow lint] .dev-doc lint report");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("Phase: {}", phase);
    println!();

    if !result.fixed.is_empty() {
        println!("Auto-fixed ({} items):", result.fixed.len());
        for item in &result.fixed {
            println!("  + {}", item);
        }
        println!();
    }

    if !result.errors.is_empty() {
        println!("Errors ({} items):", result.errors.len());
        for e in &result.errors {
            println!("  x {}", e);
        }
        println!();
    }

    if !result.warnings.is_empty() {
        println!("Warnings ({} items):", result.warnings.len());
        for w in &result.warnings {
            println!("  ! {}", w);
        }
        println!();
    }

    if !result.ok.is_empty() {
        println!("OK ({} items):", result.ok.len());
        for o in &result.ok {
            println!("  . {}", o);
        }
        println!();
    }

    if result.pass {
        println!("Result: PASS");
    } else {
        println!("Result: FAIL ({} errors)", result.errors.len());
    }
}
