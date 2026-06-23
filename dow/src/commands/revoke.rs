// dow/src/commands/
// ├── revoke.rs  -- dow revoke (iterate inverse operation: restore archive → rollback version → mark revoked)
//
// Related Docs:
// - [GitHub issue #10](https://github.com/daphnee-ovo/dev-flow/issues/10)

use crate::cli::RevokeArgs;
use crate::core::{archive_db, doc_root, version, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
struct RevokeOutput {
    revoked_version: String,
    restored_tasks: u32,
    restored_issues: u32,
    restored_docs: Vec<String>,
    phase: String,
}

#[derive(Serialize)]
struct RevokeListOutput {
    versions: Vec<RevokeListEntry>,
}

#[derive(Serialize)]
struct RevokeListEntry {
    version: String,
    topic: String,
    released_at: String,
}

pub fn run(args: RevokeArgs, human: bool) -> Result<i32, DowError> {
    if args.list {
        return run_list(human);
    }

    let target_version = args.version
        .ok_or_else(|| DowError::new("Must specify --version <version> (use --list to see available versions)", 1))?;

    let archive_base = archive_db::archive_base();
    let conn = archive_db::open_or_create(&archive_base)?;

    let iteration_id = archive_db::find_active_iteration(&conn, &target_version)?
        .ok_or_else(|| DowError::new(
            format!("Version {} has no revokable archive record (does not exist or all records are already revoked)", target_version),
            1,
        ))?;

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    // 1. Restore task files
    let tasks = archive_db::query_tasks(&conn, Some(&target_version), None)?;
    let restored_tasks = restore_tasks(&doc_root_path, &tasks)?;

    // 2. Restore issue files
    let issues = archive_db::query_issues(&conn, Some(&target_version), None)?;
    let restored_issues = restore_issues(&doc_root_path, &issues)?;

    // 3. Restore documents (PRD/SPEC/TEST/BRAINSTORM)
    let mut restored_docs = Vec::new();
    for doc_type in &["PRD", "SPEC", "TEST", "BRAINSTORM"] {
        if let Ok(Some(content)) = archive_db::get_doc(&conn, &target_version, doc_type) {
            let path = doc_root_path.join(format!("{}.md", doc_type));
            fs::write(&path, &content)
                .map_err(|e| DowError::new(format!("Failed to restore {}.md: {}", doc_type, e), 1))?;
            restored_docs.push(format!("{}.md", doc_type));
        }
    }

    // 4. Restore CHANGELOG
    let changelog_entries = archive_db::query_changelog(&conn, &target_version)?;
    if !changelog_entries.is_empty() {
        let changelog_path = doc_root_path.join("CHANGELOG.md");
        let content = rebuild_changelog(&changelog_entries);
        fs::write(&changelog_path, &content)
            .map_err(|e| DowError::new(format!("Failed to restore CHANGELOG: {}", e), 1))?;
        restored_docs.push("CHANGELOG.md".to_string());
    }

    // 5. Rollback VERSION
    version::write_current(&target_version)?;

    // 6. Rollback STATUS phase to DEV
    let status_file = doc_root_path.join("STATUS.yaml");
    let phase = "DEV";
    if status_file.exists() {
        yaml::set(&status_file, "phase", phase)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
        yaml::touch_updated(&status_file)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 7. Mark iteration as revoked
    archive_db::mark_iteration_revoked(&conn, iteration_id)?;

    let result = RevokeOutput {
        revoked_version: target_version.clone(),
        restored_tasks,
        restored_issues,
        restored_docs: restored_docs.clone(),
        phase: phase.to_string(),
    };

    if human {
        println!("[dev-flow] Version rollback completed");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        println!("Revoked version: v{}", target_version);
        println!("Restored tasks: {} items", restored_tasks);
        println!("Restored issues: {} items", restored_issues);
        if !restored_docs.is_empty() {
            println!("Restored documents: {}", restored_docs.join(", "));
        }
        println!("Phase reset: {}", phase);
        println!("Archive marked: revoked");
        println!();
        println!("Note: Git history unchanged, only workflow state and documents have been restored.");
        println!("Tip: Use `dow iterate` to re-deliver, version number {} can be reused.",
            target_version);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn run_list(human: bool) -> Result<i32, DowError> {
    let archive_base = archive_db::archive_base();
    let conn = archive_db::open_or_create(&archive_base)?;
    let iterations = archive_db::list_iterations(&conn, None)?;

    // Only show versions with active (not revoked) records
    let active: Vec<_> = iterations.into_iter().filter(|i| {
        archive_db::find_active_iteration(&conn, &i.version).unwrap_or(None).is_some()
    }).collect();

    if active.is_empty() {
        if human {
            println!("[dev-flow] No revokable versions (archive is empty or all records are already revoked)");
        } else {
            output::print_json(&RevokeListOutput { versions: vec![] });
        }
        return Ok(0);
    }

    let entries: Vec<RevokeListEntry> = active
        .iter()
        .map(|i| RevokeListEntry {
            version: i.version.clone(),
            topic: i.topic.clone(),
            released_at: i.released_at.clone(),
        })
        .collect();

    if human {
        println!("[dev-flow] Revokable versions:");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        for e in &entries {
            println!("  v{} — {} ({})", e.version, e.topic, e.released_at);
        }
    } else {
        output::print_json(&RevokeListOutput { versions: entries });
    }

    Ok(0)
}

fn restore_tasks(doc_root: &std::path::Path, tasks: &[archive_db::TaskRecord]) -> Result<u32, DowError> {
    if tasks.is_empty() {
        return Ok(0);
    }

    let task_dir = doc_root.join("task");
    fs::create_dir_all(&task_dir)
        .map_err(|e| DowError::new(format!("Failed to create task directory: {}", e), 1))?;

    // Group by source_file
    let mut groups: std::collections::HashMap<String, Vec<&archive_db::TaskRecord>> = std::collections::HashMap::new();
    for task in tasks {
        groups.entry(task.source_file.clone()).or_default().push(task);
    }

    // Restore files keeping done_task_ prefix (they are completed tasks)
    // Need to arrange seq numbers: first make room, then write
    let revoke_files: Vec<(String, String)> = groups.iter().map(|(filename, group_tasks)| {
        let content = rebuild_task_file(filename, group_tasks);
        // Keep original filename (already done_task_ or task_)
        (filename.clone(), content)
    }).collect();

    // Calculate how many seq slots needed
    let slots_needed = revoke_files.len() as u32;

    // Shift existing files, make room for slots_needed slots starting from seq=1
    shift_files_in_dir(&task_dir, slots_needed)?;

    // Write revoke files, occupying seq 1..=slots_needed
    let mut count = 0u32;
    for (i, (_orig_name, content)) in revoke_files.iter().enumerate() {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let seq = i as u32 + 1;
        let new_name = format!("done_task_{}_{}.md", today, seq);
        let path = task_dir.join(&new_name);
        fs::write(&path, content)
            .map_err(|e| DowError::new(format!("Failed to restore {}: {}", new_name, e), 1))?;
        count += groups.values().nth(i).map(|g| g.len() as u32).unwrap_or(0);
    }

    Ok(count)
}

fn restore_issues(doc_root: &std::path::Path, issues: &[archive_db::IssueRecord]) -> Result<u32, DowError> {
    if issues.is_empty() {
        return Ok(0);
    }

    let issue_dir = doc_root.join("issue");
    fs::create_dir_all(&issue_dir)
        .map_err(|e| DowError::new(format!("Failed to create issue directory: {}", e), 1))?;

    let mut groups: std::collections::HashMap<String, Vec<&archive_db::IssueRecord>> = std::collections::HashMap::new();
    for issue in issues {
        groups.entry(issue.source_file.clone()).or_default().push(issue);
    }

    let revoke_files: Vec<(String, String)> = groups.iter().map(|(filename, group_issues)| {
        let content = rebuild_issue_file(filename, group_issues);
        (filename.clone(), content)
    }).collect();

    let slots_needed = revoke_files.len() as u32;
    shift_files_in_dir(&issue_dir, slots_needed)?;

    let mut count = 0u32;
    for (i, (_orig_name, content)) in revoke_files.iter().enumerate() {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let seq = i as u32 + 1;
        let new_name = format!("closed_issue_{}_{}.md", today, seq);
        let path = issue_dir.join(&new_name);
        fs::write(&path, content)
            .map_err(|e| DowError::new(format!("Failed to restore {}: {}", new_name, e), 1))?;
        count += groups.values().nth(i).map(|g| g.len() as u32).unwrap_or(0);
    }

    Ok(count)
}

/// Shift seq numbers of .md files in directory, make room for first slots_needed positions
/// For example, if directory has _1.md, _2.md, _3.md, and slots_needed=2, rename them to _3.md, _4.md, _5.md
fn shift_files_in_dir(dir: &std::path::Path, slots_needed: u32) -> Result<(), DowError> {
    if slots_needed == 0 {
        return Ok(());
    }

    // Collect all .md files and their seq numbers
    let mut files_with_seq: Vec<(String, u32)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Some(seq) = extract_file_seq(&name) {
                files_with_seq.push((name, seq));
            }
        }
    }

    if files_with_seq.is_empty() {
        return Ok(());
    }

    // Sort from large to small to avoid rename conflicts
    files_with_seq.sort_by(|a, b| b.1.cmp(&a.1));

    for (name, seq) in &files_with_seq {
        let new_seq = seq + slots_needed;
        let new_name = replace_file_seq(&name, *seq, new_seq);
        let old_path = dir.join(name);
        let new_path = dir.join(&new_name);
        fs::rename(&old_path, &new_path)
            .map_err(|e| DowError::new(format!("Failed to shift {} → {}: {}", name, new_name, e), 1))?;
    }

    Ok(())
}

/// Extract seq number from end of filename: xxx_3.md → 3
fn extract_file_seq(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".md")?;
    let last_part = stem.rsplit('_').next()?;
    last_part.parse().ok()
}

/// Replace seq number in filename
fn replace_file_seq(name: &str, old_seq: u32, new_seq: u32) -> String {
    let old_suffix = format!("_{}.md", old_seq);
    let new_suffix = format!("_{}.md", new_seq);
    if name.ends_with(&old_suffix) {
        format!("{}{}", &name[..name.len() - old_suffix.len()], new_suffix)
    } else {
        name.to_string()
    }
}

fn rebuild_task_file(filename: &str, tasks: &[&archive_db::TaskRecord]) -> String {
    let title = tasks.first()
        .and_then(|t| t.file_title.as_deref())
        .unwrap_or("TASK - (revoked)");

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", title));
    out.push_str(&format!("nums: {}\n", tasks.len()));
    out.push_str("---\n\n");

    for task in tasks {
        let check = if task.completed { "[x]" } else { "[ ]" };
        out.push_str(&format!("- {} {}: {}\n", check, task.task_id, task.title));

        if let Some(ref p) = task.priority {
            out.push_str(&format!("  - priority: {}\n", p));
        }
        if let Some(ref r) = task.refs {
            out.push_str(&format!("  - refs: {}\n", r));
        }
        out.push_str("  - files:\n");
        out.push_str(&format!("      create: {}\n", task.files_create.as_deref().unwrap_or("[]")));
        out.push_str(&format!("      modify: {}\n", task.files_modify.as_deref().unwrap_or("[]")));
        out.push_str(&format!("      test: {}\n", task.files_test.as_deref().unwrap_or("[]")));
        out.push_str(&format!("  - depends_on: {}\n", task.depends_on.as_deref().unwrap_or("[]")));
        if let Some(ref c) = task.complexity {
            out.push_str(&format!("  - complexity: {}\n", c));
        }
        if let Some(ref dw) = task.done_when {
            out.push_str(&format!("  - done_when: {}\n", dw));
        }
        out.push('\n');
    }

    out
}

fn rebuild_issue_file(_filename: &str, issues: &[&archive_db::IssueRecord]) -> String {
    let source_type = issues.first()
        .and_then(|i| i.source_type.as_deref())
        .unwrap_or("revoke");

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("source: {}\n", source_type));
    out.push_str(&format!("nums: {}\n", issues.len()));
    out.push_str("---\n\n");

    for issue in issues {
        let check = if issue.resolved { "[x]" } else { "[ ]" };
        out.push_str(&format!("- {} {}: {}\n", check, issue.issue_id, issue.title));

        if let Some(ref s) = issue.severity {
            out.push_str(&format!("  - severity: {}\n", s));
        }
        if let Some(ref l) = issue.location {
            out.push_str(&format!("  - location: {}\n", l));
        }
        if let Some(ref d) = issue.description {
            out.push_str(&format!("  - description: {}\n", d));
        }
        if let Some(ref e) = issue.expected {
            out.push_str(&format!("  - expected: {}\n", e));
        }
        if let Some(ref a) = issue.actual {
            out.push_str(&format!("  - actual: {}\n", a));
        }
        if let Some(ref r) = issue.reproduce {
            out.push_str(&format!("  - reproduce: {}\n", r));
        }
        if let Some(ref f) = issue.fix {
            out.push_str(&format!("  - fix: {}\n", f));
        }
        out.push('\n');
    }

    out
}

fn rebuild_changelog(entries: &[(Option<String>, String)]) -> String {
    let mut out = String::from("# Changelog\n\n");
    let mut current_date: Option<&str> = None;

    for (date, text) in entries {
        let d = date.as_deref().unwrap_or("");
        if Some(d) != current_date {
            if current_date.is_some() {
                out.push('\n');
            }
            out.push_str(&format!("## {}\n", d));
            current_date = Some(d);
        }
        out.push_str(&format!("- {}\n", text));
    }

    out
}
