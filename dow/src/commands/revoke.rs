// dow/src/commands/
// ├── revoke.rs  -- dow revoke（iterate 逆操作：还原归档 → 回退版本 → 标记 revoked）
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
        .ok_or_else(|| DowError::new("必须指定 --version <版本号>（使用 --list 查看可选版本）", 1))?;

    let archive_base = archive_db::archive_base();
    let conn = archive_db::open_or_create(&archive_base)?;

    let iteration_id = archive_db::find_active_iteration(&conn, &target_version)?
        .ok_or_else(|| DowError::new(
            format!("版本 {} 无可回退的归档记录（不存在或已全部 revoked）", target_version),
            1,
        ))?;

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    // 1. 还原 task 文件
    let tasks = archive_db::query_tasks(&conn, Some(&target_version), None)?;
    let restored_tasks = restore_tasks(&doc_root_path, &tasks)?;

    // 2. 还原 issue 文件
    let issues = archive_db::query_issues(&conn, Some(&target_version), None)?;
    let restored_issues = restore_issues(&doc_root_path, &issues)?;

    // 3. 还原文档（PRD/SPEC/TEST/BRAINSTORM）
    let mut restored_docs = Vec::new();
    for doc_type in &["PRD", "SPEC", "TEST", "BRAINSTORM"] {
        if let Ok(Some(content)) = archive_db::get_doc(&conn, &target_version, doc_type) {
            let path = doc_root_path.join(format!("{}.md", doc_type));
            fs::write(&path, &content)
                .map_err(|e| DowError::new(format!("还原 {}.md 失败：{}", doc_type, e), 1))?;
            restored_docs.push(format!("{}.md", doc_type));
        }
    }

    // 4. 还原 CHANGELOG
    let changelog_entries = archive_db::query_changelog(&conn, &target_version)?;
    if !changelog_entries.is_empty() {
        let changelog_path = doc_root_path.join("CHANGELOG.md");
        let content = rebuild_changelog(&changelog_entries);
        fs::write(&changelog_path, &content)
            .map_err(|e| DowError::new(format!("还原 CHANGELOG 失败：{}", e), 1))?;
        restored_docs.push("CHANGELOG.md".to_string());
    }

    // 5. 回退 VERSION
    version::write_current(&target_version)?;

    // 6. 回退 STATUS phase 到 DEV
    let status_file = doc_root_path.join("STATUS.yaml");
    let phase = "DEV";
    if status_file.exists() {
        yaml::set(&status_file, "phase", phase)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
        yaml::touch_updated(&status_file)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 7. 标记 iteration 为 revoked
    archive_db::mark_iteration_revoked(&conn, iteration_id)?;

    let result = RevokeOutput {
        revoked_version: target_version.clone(),
        restored_tasks,
        restored_issues,
        restored_docs: restored_docs.clone(),
        phase: phase.to_string(),
    };

    if human {
        println!("[dev-flow] 版本回退完成");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        println!("回退版本：v{}", target_version);
        println!("还原 task：{} 个", restored_tasks);
        println!("还原 issue：{} 个", restored_issues);
        if !restored_docs.is_empty() {
            println!("还原文档：{}", restored_docs.join(", "));
        }
        println!("阶段重置：{}", phase);
        println!("归档标记：revoked");
        println!();
        println!("注意：git 历史未变更，仅流程状态与文档已还原。");
        println!("提示：使用 `dow iterate` 重新交付，版本号 {} 可复用。",
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

    // 只展示有活跃（未 revoked）记录的版本
    let active: Vec<_> = iterations.into_iter().filter(|i| {
        archive_db::find_active_iteration(&conn, &i.version).unwrap_or(None).is_some()
    }).collect();

    if active.is_empty() {
        if human {
            println!("[dev-flow] 无可回退版本（归档为空或全部已 revoked）");
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
        println!("[dev-flow] 可回退版本：");
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
        .map_err(|e| DowError::new(format!("创建 task 目录失败：{}", e), 1))?;

    // 按 source_file 分组
    let mut groups: std::collections::HashMap<String, Vec<&archive_db::TaskRecord>> = std::collections::HashMap::new();
    for task in tasks {
        groups.entry(task.source_file.clone()).or_default().push(task);
    }

    // 还原文件保持 done_task_ 前缀（它们是已完成的 task）
    // 需要安排 seq 号：先腾出位置，再写入
    let revoke_files: Vec<(String, String)> = groups.iter().map(|(filename, group_tasks)| {
        let content = rebuild_task_file(filename, group_tasks);
        // 保持原始文件名（已经是 done_task_ 或 task_）
        (filename.clone(), content)
    }).collect();

    // 计算需要多少个 seq slot
    let slots_needed = revoke_files.len() as u32;

    // 顺延现有文件，从 seq=1 开始腾出 slots_needed 个位置
    shift_files_in_dir(&task_dir, slots_needed)?;

    // 写入 revoke 文件，占据 seq 1..=slots_needed
    let mut count = 0u32;
    for (i, (_orig_name, content)) in revoke_files.iter().enumerate() {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let seq = i as u32 + 1;
        let new_name = format!("done_task_{}_{}.md", today, seq);
        let path = task_dir.join(&new_name);
        fs::write(&path, content)
            .map_err(|e| DowError::new(format!("还原 {} 失败：{}", new_name, e), 1))?;
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
        .map_err(|e| DowError::new(format!("创建 issue 目录失败：{}", e), 1))?;

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
            .map_err(|e| DowError::new(format!("还原 {} 失败：{}", new_name, e), 1))?;
        count += groups.values().nth(i).map(|g| g.len() as u32).unwrap_or(0);
    }

    Ok(count)
}

/// 顺延目录中的 .md 文件 seq 号，腾出前 slots_needed 个位置
/// 例如目录中有 _1.md, _2.md, _3.md，slots_needed=2，则重命名为 _3.md, _4.md, _5.md
fn shift_files_in_dir(dir: &std::path::Path, slots_needed: u32) -> Result<(), DowError> {
    if slots_needed == 0 {
        return Ok(());
    }

    // 收集所有 .md 文件及其 seq 号
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

    // 从大到小排序，避免重命名覆盖
    files_with_seq.sort_by(|a, b| b.1.cmp(&a.1));

    for (name, seq) in &files_with_seq {
        let new_seq = seq + slots_needed;
        let new_name = replace_file_seq(&name, *seq, new_seq);
        let old_path = dir.join(name);
        let new_path = dir.join(&new_name);
        fs::rename(&old_path, &new_path)
            .map_err(|e| DowError::new(format!("顺延 {} → {} 失败：{}", name, new_name, e), 1))?;
    }

    Ok(())
}

/// 从文件名末尾提取 seq 号：xxx_3.md → 3
fn extract_file_seq(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".md")?;
    let last_part = stem.rsplit('_').next()?;
    last_part.parse().ok()
}

/// 替换文件名中的 seq 号
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
