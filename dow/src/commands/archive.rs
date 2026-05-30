// dow/src/commands/
// ├── archive.rs  -- dow archive 子命令（list/show/tasks/issues/doc/migrate/stats）

use crate::cli::ArchiveCommands;
use crate::core::archive_db;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub fn run(cmd: ArchiveCommands, human: bool) -> Result<i32, DowError> {
    match cmd {
        ArchiveCommands::List { branch } => run_list(branch.as_deref(), human),
        ArchiveCommands::Show { version } => run_show(&version, human),
        ArchiveCommands::Tasks { version, priority } => run_tasks(version.as_deref(), priority.as_deref(), human),
        ArchiveCommands::Issues { version, severity } => run_issues(version.as_deref(), severity.as_deref(), human),
        ArchiveCommands::Doc { version, doc_type } => run_doc(&version, &doc_type, human),
        ArchiveCommands::Migrate { delete_originals } => run_migrate(delete_originals, human),
        ArchiveCommands::Stats => run_stats(human),
    }
}

// ─── list ───

#[derive(Serialize)]
struct ListItem {
    version: String,
    topic: String,
    branch: String,
    released_at: String,
    tasks: i64,
    issues: i64,
}

fn run_list(branch: Option<&str>, human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;
    let items = archive_db::list_iterations(&conn, branch)?;

    if human {
        println!("[archive] 共 {} 个版本", items.len());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for it in &items {
            println!(
                "  v{:<10} {:<30} [{} tasks, {} issues] ({})",
                it.version, it.topic, it.task_count, it.issue_count, it.released_at
            );
        }
    } else {
        let list: Vec<ListItem> = items
            .into_iter()
            .map(|it| ListItem {
                version: it.version,
                topic: it.topic,
                branch: it.branch,
                released_at: it.released_at,
                tasks: it.task_count,
                issues: it.issue_count,
            })
            .collect();
        output::print_json(&list);
    }
    Ok(0)
}

// ─── show ───

#[derive(Serialize)]
struct ShowOutput {
    version: String,
    topic: String,
    tasks: Vec<TaskBrief>,
    issues: Vec<IssueBrief>,
    has_prd: bool,
    has_spec: bool,
    has_test: bool,
    has_brainstorm: bool,
}

#[derive(Serialize)]
struct TaskBrief {
    task_id: String,
    title: String,
    priority: Option<String>,
    completed: bool,
}

#[derive(Serialize)]
struct IssueBrief {
    issue_id: String,
    title: String,
    severity: Option<String>,
    resolved: bool,
}

fn run_show(version: &str, human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;

    let tasks = archive_db::query_tasks(&conn, Some(version), None)?;
    let issues = archive_db::query_issues(&conn, Some(version), None)?;
    let has_prd = archive_db::get_doc(&conn, version, "PRD")?.is_some();
    let has_spec = archive_db::get_doc(&conn, version, "SPEC")?.is_some();
    let has_test = archive_db::get_doc(&conn, version, "TEST")?.is_some();
    let has_brainstorm = archive_db::get_doc(&conn, version, "BRAINSTORM")?.is_some();

    // 获取 topic
    let iterations = archive_db::list_iterations(&conn, None)?;
    let topic = iterations
        .iter()
        .find(|i| i.version == version)
        .map(|i| i.topic.clone())
        .unwrap_or_default();

    if human {
        println!("[archive] v{} — {}", version, topic);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("文档：PRD={} SPEC={} TEST={} BRAINSTORM={}", yn(has_prd), yn(has_spec), yn(has_test), yn(has_brainstorm));
        if !tasks.is_empty() {
            println!("\n任务（{}个）：", tasks.len());
            for t in &tasks {
                let mark = if t.completed { "x" } else { " " };
                let p = t.priority.as_deref().unwrap_or("-");
                println!("  [{}] {} {} ({})", mark, t.task_id, t.title, p);
            }
        }
        if !issues.is_empty() {
            println!("\nIssue（{}个）：", issues.len());
            for iss in &issues {
                let mark = if iss.resolved { "x" } else { " " };
                let s = iss.severity.as_deref().unwrap_or("-");
                println!("  [{}] {} {} ({})", mark, iss.issue_id, iss.title, s);
            }
        }
    } else {
        let out = ShowOutput {
            version: version.to_string(),
            topic,
            tasks: tasks.iter().map(|t| TaskBrief {
                task_id: t.task_id.clone(),
                title: t.title.clone(),
                priority: t.priority.clone(),
                completed: t.completed,
            }).collect(),
            issues: issues.iter().map(|i| IssueBrief {
                issue_id: i.issue_id.clone(),
                title: i.title.clone(),
                severity: i.severity.clone(),
                resolved: i.resolved,
            }).collect(),
            has_prd,
            has_spec,
            has_test,
            has_brainstorm,
        };
        output::print_json(&out);
    }
    Ok(0)
}

// ─── tasks ───

fn run_tasks(version: Option<&str>, priority: Option<&str>, human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;
    let tasks = archive_db::query_tasks(&conn, version, priority)?;

    if human {
        println!("[archive] 任务查询：{} 条", tasks.len());
        for t in &tasks {
            let mark = if t.completed { "x" } else { " " };
            let p = t.priority.as_deref().unwrap_or("-");
            println!("  [{}] {} {} ({})", mark, t.task_id, t.title, p);
        }
    } else {
        let briefs: Vec<TaskBrief> = tasks.iter().map(|t| TaskBrief {
            task_id: t.task_id.clone(),
            title: t.title.clone(),
            priority: t.priority.clone(),
            completed: t.completed,
        }).collect();
        output::print_json(&briefs);
    }
    Ok(0)
}

// ─── issues ───

fn run_issues(version: Option<&str>, severity: Option<&str>, human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;
    let issues = archive_db::query_issues(&conn, version, severity)?;

    if human {
        println!("[archive] Issue 查询：{} 条", issues.len());
        for iss in &issues {
            let mark = if iss.resolved { "x" } else { " " };
            let s = iss.severity.as_deref().unwrap_or("-");
            println!("  [{}] {} {} ({})", mark, iss.issue_id, iss.title, s);
        }
    } else {
        let briefs: Vec<IssueBrief> = issues.iter().map(|i| IssueBrief {
            issue_id: i.issue_id.clone(),
            title: i.title.clone(),
            severity: i.severity.clone(),
            resolved: i.resolved,
        }).collect();
        output::print_json(&briefs);
    }
    Ok(0)
}

// ─── doc ───

fn run_doc(version: &str, doc_type: &str, _human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;
    let dtype = doc_type.to_uppercase();
    match archive_db::get_doc(&conn, version, &dtype)? {
        Some(content) => {
            println!("{}", content);
            Ok(0)
        }
        None => Err(DowError::new(
            format!("v{} 没有 {} 文档", version, dtype),
            1,
        )),
    }
}

// ─── migrate ───

#[derive(Serialize)]
struct MigrateOutput {
    migrated: u32,
    versions: Vec<String>,
    total_records: u32,
}

fn run_migrate(delete_originals: bool, human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;

    let mut versions = Vec::new();
    let mut total_records = 0u32;

    // 扫描 .dev-doc/archive/v*-*/
    let top_archive = doc_root_path.join("archive");
    if top_archive.is_dir() {
        migrate_dir_entries(&conn, &top_archive, "main", &mut versions, &mut total_records)?;
    }

    // 扫描 .dev-doc/*/archive/v*-*/（branch-specific）
    if let Ok(entries) = fs::read_dir(&doc_root_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() && name != "archive" {
                let branch_archive = entry.path().join("archive");
                if branch_archive.is_dir() {
                    migrate_dir_entries(&conn, &branch_archive, &name, &mut versions, &mut total_records)?;
                }
            }
        }
    }

    if delete_originals && !versions.is_empty() {
        // 删除顶层 archive 目录
        if top_archive.is_dir() {
            fs::remove_dir_all(&top_archive).ok();
        }
        // 删除 branch-specific archive 目录
        if let Ok(entries) = fs::read_dir(&doc_root_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().is_dir() && name != "archive" {
                    let branch_archive = entry.path().join("archive");
                    if branch_archive.is_dir() {
                        fs::remove_dir_all(&branch_archive).ok();
                    }
                }
            }
        }
    }

    let result = MigrateOutput {
        migrated: versions.len() as u32,
        versions: versions.clone(),
        total_records,
    };

    if human {
        println!("[archive] 迁移完成");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  版本数：{}", versions.len());
        println!("  记录数：{}", total_records);
        for v in &versions {
            println!("  - v{}", v);
        }
        if delete_originals {
            println!("  [已删除原始目录]");
        }
    } else {
        output::print_json(&result);
    }
    Ok(0)
}

fn migrate_dir_entries(
    conn: &rusqlite::Connection,
    archive_parent: &Path,
    branch: &str,
    versions: &mut Vec<String>,
    total_records: &mut u32,
) -> Result<(), DowError> {
    if let Ok(entries) = fs::read_dir(archive_parent) {
        let mut dirs: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();
        dirs.sort_by_key(|e| e.file_name());

        for entry in dirs {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if let Some((version, topic)) = archive_db::parse_archive_dir_name(&dir_name) {
                // 跳过已存在的版本
                let existing = archive_db::list_iterations(conn, None)?;
                if existing.iter().any(|i| i.version == version) {
                    continue;
                }
                let count = archive_db::migrate_archive_dir(conn, &entry.path(), &version, &topic, branch)?;
                versions.push(version);
                *total_records += count;
            }
        }
    }
    Ok(())
}

// ─── stats ───

fn run_stats(human: bool) -> Result<i32, DowError> {
    let doc_root_path = archive_db::archive_base();
    let conn = archive_db::open_or_create(&doc_root_path)?;
    let stats = archive_db::get_stats(&conn)?;

    if human {
        println!("[archive] 统计");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  版本数：{}", stats["iterations"]);
        println!("  任务数：{}", stats["tasks"]);
        println!("  Issue 数：{}", stats["issues"]);
    } else {
        output::print_json(&stats);
    }
    Ok(0)
}

fn yn(b: bool) -> &'static str {
    if b { "✓" } else { "✗" }
}
