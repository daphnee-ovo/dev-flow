// dow/src/commands/
// ├── check.rs  -- dow check (document specification check)

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct CheckOutput {
    pass: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    ok: Vec<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new("STATUS.yaml not found", 1));
    }

    let map = yaml::read(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;
    let phase = map.get("phase").cloned().unwrap_or_default();
    let mode = map.get("mode").cloned().unwrap_or_default();

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ok = Vec::new();

    // 1. CHANGELOG check
    check_changelog(&doc_root_path, &mut warnings, &mut ok);

    // 2. task completion vs phase matching
    let (total, done) = check_tasks(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);

    // 3. issue status check
    check_issues(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);

    // 4. code changes vs document update time
    check_time_sync(&status_file, &map, &mut warnings, &mut ok);

    // 5. phase required files
    check_phase_files(&doc_root_path, &phase, &mut warnings);

    // 6. SPEC acceptance check
    check_spec_ac(&doc_root_path, &mode, &mut errors, &mut warnings);

    // 7. TEST report check
    check_test_report(&doc_root_path, total, done, &mut warnings);

    // 8. task nums consistency
    check_task_nums(&doc_root_path, &mut errors);

    let pass = errors.is_empty();

    let result = CheckOutput { pass, errors, warnings, ok };

    if human {
        print_human(&result, &phase);
    } else {
        output::print_json(&result);
    }

    Ok(if pass { 0 } else { 1 })
}

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

    // task nums declaration vs actual
    let _ = errors; // nums check is in separate function

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
                // Find nums: declaration
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
    _status_file: &Path,
    map: &std::collections::BTreeMap<String, String>,
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
                    errors.push(format!("spec_missing_ac: {} mode SPEC missing testable acceptance criteria", mode));
                }
                _ => {
                    warnings.push("SPEC lacks explicit acceptance criteria, consider adding them".to_string());
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
                || content.contains("FAILED:")
                || content.contains("NOT PASSED:")
                || content.contains("NOT PASSED:");
            if has_failure {
                warnings.push("TEST.md reports failed tests".to_string());
            }
        }
    }
}

fn print_human(result: &CheckOutput, phase: &str) {
    println!("[dev-flow] Document Sync Check");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("Current Phase: {}", phase);
    println!();

    if !result.warnings.is_empty() {
        println!("⚠ Warnings ({} items):", result.warnings.len());
        for w in &result.warnings {
            println!("  - {}", w);
        }
        println!();
    }

    if !result.errors.is_empty() {
        println!("✗ Blocking Errors ({} items):", result.errors.len());
        for e in &result.errors {
            println!("  - {}", e);
        }
        println!();
    }

    if !result.ok.is_empty() {
        println!("✓ OK ({} items):", result.ok.len());
        for o in &result.ok {
            println!("  - {}", o);
        }
    }

    if result.warnings.is_empty() && result.errors.is_empty() {
        println!("✓ Document sync status is good, no action needed.");
    }
}
