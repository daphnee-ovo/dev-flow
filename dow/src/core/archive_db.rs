// dow/src/core/
// ├── archive_db.rs  -- SQLite 归档存储（建表、读写、解析）

use crate::error::DowError;
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

// ─── 数据结构 ───

pub struct IterationRecord {
    pub version: String,
    pub topic: String,
    pub commit_type: Option<String>,
    pub branch: String,
    pub released_at: String,
    pub tag: String,
    pub mode: Option<String>,
}

pub struct TaskRecord {
    pub source_file: String,
    pub file_title: Option<String>,
    pub task_id: String,
    pub title: String,
    pub completed: bool,
    pub priority: Option<String>,
    pub refs: Option<String>,
    pub files_create: Option<String>,
    pub files_modify: Option<String>,
    pub files_test: Option<String>,
    pub depends_on: Option<String>,
    pub complexity: Option<String>,
    pub done_when: Option<String>,
}

pub struct IssueRecord {
    pub source_file: String,
    pub source_type: Option<String>,
    pub issue_id: String,
    pub title: String,
    pub resolved: bool,
    pub severity: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub reproduce: Option<String>,
    pub fix: Option<String>,
}

// ─── 连接与建表 ───

/// 打开归档数据库（始终在 .dev-doc/archive.db，跨分支共享）
pub fn open_or_create(base: &Path) -> Result<Connection, DowError> {
    let db_path = base.join("archive.db");
    let conn = Connection::open(&db_path)
        .map_err(|e| DowError::new(format!("打开 archive.db 失败：{}", e), 1))?;

    conn.execute_batch("PRAGMA journal_mode=DELETE;")
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    create_tables(&conn)?;
    Ok(conn)
}

/// 归档数据库基础路径（固定 .dev-doc/，不随分支变化）
pub fn archive_base() -> std::path::PathBuf {
    std::path::PathBuf::from(crate::core::DOC_DIR)
}

fn create_tables(conn: &Connection) -> Result<(), DowError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS iterations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL UNIQUE,
            topic TEXT NOT NULL,
            commit_type TEXT,
            branch TEXT NOT NULL DEFAULT 'main',
            released_at TEXT NOT NULL,
            tag TEXT NOT NULL,
            mode TEXT
        );

        CREATE TABLE IF NOT EXISTS prd_docs (
            version TEXT PRIMARY KEY,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS spec_docs (
            version TEXT PRIMARY KEY,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS test_docs (
            version TEXT PRIMARY KEY,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS brainstorm_docs (
            version TEXT PRIMARY KEY,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS changelog_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            entry_date TEXT,
            entry_text TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            source_file TEXT NOT NULL,
            file_title TEXT,
            task_id TEXT NOT NULL,
            title TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 1,
            priority TEXT,
            refs TEXT,
            files_create TEXT,
            files_modify TEXT,
            files_test TEXT,
            depends_on TEXT,
            complexity TEXT,
            done_when TEXT
        );

        CREATE TABLE IF NOT EXISTS issues (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            source_file TEXT NOT NULL,
            source_type TEXT,
            issue_id TEXT NOT NULL,
            title TEXT NOT NULL,
            resolved INTEGER NOT NULL DEFAULT 1,
            severity TEXT,
            location TEXT,
            description TEXT,
            expected TEXT,
            actual TEXT,
            reproduce TEXT,
            fix TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_iterations_version ON iterations(version);
        CREATE INDEX IF NOT EXISTS idx_changelog_version ON changelog_entries(version);
        CREATE INDEX IF NOT EXISTS idx_tasks_version ON tasks(version);
        CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
        CREATE INDEX IF NOT EXISTS idx_issues_version ON issues(version);
        CREATE INDEX IF NOT EXISTS idx_issues_severity ON issues(severity);

        INSERT OR IGNORE INTO schema_version(version) VALUES(1);
        ",
    )
    .map_err(|e| DowError::new(format!("建表失败：{}", e), 1))?;
    Ok(())
}

// ─── 写入 ───

pub fn insert_iteration(conn: &Connection, rec: &IterationRecord) -> Result<(), DowError> {
    conn.execute(
        "INSERT OR IGNORE INTO iterations (version, topic, commit_type, branch, released_at, tag, mode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            rec.version,
            rec.topic,
            rec.commit_type,
            rec.branch,
            rec.released_at,
            rec.tag,
            rec.mode,
        ],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn insert_doc(conn: &Connection, version: &str, doc_type: &str, content: &str) -> Result<(), DowError> {
    let sql = match doc_type {
        "PRD" => "INSERT OR REPLACE INTO prd_docs (version, content) VALUES (?1, ?2)",
        "SPEC" => "INSERT OR REPLACE INTO spec_docs (version, content) VALUES (?1, ?2)",
        "TEST" => "INSERT OR REPLACE INTO test_docs (version, content) VALUES (?1, ?2)",
        "BRAINSTORM" => "INSERT OR REPLACE INTO brainstorm_docs (version, content) VALUES (?1, ?2)",
        _ => return Err(DowError::new(format!("未知文档类型：{}", doc_type), 1)),
    };
    conn.execute(sql, params![version, content])
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn insert_task(conn: &Connection, version: &str, task: &TaskRecord) -> Result<(), DowError> {
    conn.execute(
        "INSERT INTO tasks (version, source_file, file_title, task_id, title, completed,
         priority, refs, files_create, files_modify, files_test, depends_on, complexity, done_when)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            version,
            task.source_file,
            task.file_title,
            task.task_id,
            task.title,
            task.completed as i32,
            task.priority,
            task.refs,
            task.files_create,
            task.files_modify,
            task.files_test,
            task.depends_on,
            task.complexity,
            task.done_when,
        ],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn insert_issue(conn: &Connection, version: &str, issue: &IssueRecord) -> Result<(), DowError> {
    conn.execute(
        "INSERT INTO issues (version, source_file, source_type, issue_id, title, resolved,
         severity, location, description, expected, actual, reproduce, fix)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            version,
            issue.source_file,
            issue.source_type,
            issue.issue_id,
            issue.title,
            issue.resolved as i32,
            issue.severity,
            issue.location,
            issue.description,
            issue.expected,
            issue.actual,
            issue.reproduce,
            issue.fix,
        ],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn insert_changelog(conn: &Connection, version: &str, date: Option<&str>, text: &str, order: i32) -> Result<(), DowError> {
    conn.execute(
        "INSERT INTO changelog_entries (version, entry_date, entry_text, sort_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![version, date, text, order],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

// ─── 查询 ───

pub struct IterationSummary {
    pub version: String,
    pub topic: String,
    pub branch: String,
    pub released_at: String,
    pub tag: String,
    pub task_count: i64,
    pub issue_count: i64,
}

pub fn list_iterations(conn: &Connection, branch: Option<&str>) -> Result<Vec<IterationSummary>, DowError> {
    let mut sql = String::from(
        "SELECT i.version, i.topic, i.branch, i.released_at, i.tag,
         (SELECT COUNT(*) FROM tasks t WHERE t.version = i.version) as task_count,
         (SELECT COUNT(*) FROM issues s WHERE s.version = i.version) as issue_count
         FROM iterations i"
    );
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(b) = branch {
        sql.push_str(" WHERE i.branch = ?1");
        bind_values.push(b.to_string());
    }
    sql.push_str(" ORDER BY i.id ASC");

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params_slice.as_slice(), |row| {
        Ok(IterationSummary {
            version: row.get(0)?,
            topic: row.get(1)?,
            branch: row.get(2)?,
            released_at: row.get(3)?,
            tag: row.get(4)?,
            task_count: row.get(5)?,
            issue_count: row.get(6)?,
        })
    }).map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn get_doc(conn: &Connection, version: &str, doc_type: &str) -> Result<Option<String>, DowError> {
    let sql = match doc_type {
        "PRD" => "SELECT content FROM prd_docs WHERE version = ?1",
        "SPEC" => "SELECT content FROM spec_docs WHERE version = ?1",
        "TEST" => "SELECT content FROM test_docs WHERE version = ?1",
        "BRAINSTORM" => "SELECT content FROM brainstorm_docs WHERE version = ?1",
        _ => return Err(DowError::new(format!("未知文档类型：{}", doc_type), 1)),
    };
    let result = conn
        .query_row(sql, params![version], |row| row.get::<_, String>(0))
        .ok();
    Ok(result)
}

pub fn query_tasks(conn: &Connection, version: Option<&str>, priority: Option<&str>) -> Result<Vec<TaskRecord>, DowError> {
    let mut sql = String::from(
        "SELECT source_file, file_title, task_id, title, completed, priority, refs,
         files_create, files_modify, files_test, depends_on, complexity, done_when
         FROM tasks WHERE 1=1"
    );
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(v) = version {
        bind_values.push(v.to_string());
        sql.push_str(&format!(" AND version = ?{}", bind_values.len()));
    }
    if let Some(p) = priority {
        bind_values.push(p.to_string());
        sql.push_str(&format!(" AND priority = ?{}", bind_values.len()));
    }
    sql.push_str(" ORDER BY id ASC");

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params_slice.as_slice(), |row| {
        Ok(TaskRecord {
            source_file: row.get(0)?,
            file_title: row.get(1)?,
            task_id: row.get(2)?,
            title: row.get(3)?,
            completed: row.get::<_, i32>(4)? != 0,
            priority: row.get(5)?,
            refs: row.get(6)?,
            files_create: row.get(7)?,
            files_modify: row.get(8)?,
            files_test: row.get(9)?,
            depends_on: row.get(10)?,
            complexity: row.get(11)?,
            done_when: row.get(12)?,
        })
    }).map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn query_issues(conn: &Connection, version: Option<&str>, severity: Option<&str>) -> Result<Vec<IssueRecord>, DowError> {
    let mut sql = String::from(
        "SELECT source_file, source_type, issue_id, title, resolved, severity,
         location, description, expected, actual, reproduce, fix
         FROM issues WHERE 1=1"
    );
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(v) = version {
        bind_values.push(v.to_string());
        sql.push_str(&format!(" AND version = ?{}", bind_values.len()));
    }
    if let Some(s) = severity {
        bind_values.push(s.to_string());
        sql.push_str(&format!(" AND severity = ?{}", bind_values.len()));
    }
    sql.push_str(" ORDER BY id ASC");

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params_slice.as_slice(), |row| {
        Ok(IssueRecord {
            source_file: row.get(0)?,
            source_type: row.get(1)?,
            issue_id: row.get(2)?,
            title: row.get(3)?,
            resolved: row.get::<_, i32>(4)? != 0,
            severity: row.get(5)?,
            location: row.get(6)?,
            description: row.get(7)?,
            expected: row.get(8)?,
            actual: row.get(9)?,
            reproduce: row.get(10)?,
            fix: row.get(11)?,
        })
    }).map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn get_stats(conn: &Connection) -> Result<serde_json::Value, DowError> {
    let iter_count: i64 = conn.query_row("SELECT COUNT(*) FROM iterations", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    let issue_count: i64 = conn.query_row("SELECT COUNT(*) FROM issues", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    Ok(serde_json::json!({
        "iterations": iter_count,
        "tasks": task_count,
        "issues": issue_count,
    }))
}

// ─── 解析逻辑 ───

/// 从 task 文件内容解析为 TaskRecord 列表（兼容新旧格式，旧格式统一转换）
pub fn parse_task_file(filename: &str, content: &str) -> Vec<TaskRecord> {
    let mut tasks = Vec::new();
    let file_title = parse_frontmatter_field(content, "title");

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut task_seq = 0;

    while i < lines.len() {
        let line = lines[i];
        // 匹配 task 条目行：`- [x] TASK-T001: xxx` 或 `- [x] T1：xxx`
        if line.starts_with("- [") {
            task_seq += 1;
            let completed = line.starts_with("- [x]");
            let after_bracket = if completed {
                &line[6..]
            } else {
                &line[5..]
            };
            let after_bracket = after_bracket.trim();

            // 解析 task_id 和 title
            let (raw_id, title) = split_id_title(after_bracket);
            let task_id = normalize_task_id(&raw_id, task_seq);

            // 收集子字段
            let mut priority = None;
            let mut refs = None;
            let mut files_create = None;
            let mut files_modify = None;
            let mut files_test = None;
            let mut depends_on = None;
            let mut complexity = None;
            let mut done_when = None;

            i += 1;
            while i < lines.len() && !lines[i].starts_with("- [") {
                let sub = lines[i].trim();
                if sub.starts_with("- ") {
                    let field = &sub[2..];
                    if let Some(val) = strip_field(field, "priority:") {
                        priority = Some(val);
                    } else if let Some(val) = strip_field(field, "level:") {
                        // 旧格式 → 转为 priority
                        priority = Some(val);
                    } else if let Some(val) = strip_field(field, "refs:") {
                        refs = Some(val);
                    } else if let Some(val) = strip_field(field, "complexity:") {
                        complexity = Some(val);
                    } else if field.starts_with("files:") {
                        // 解析 files 子块
                        i += 1;
                        while i < lines.len() {
                            let fl = lines[i].trim();
                            if fl.starts_with("create:") {
                                files_create = Some(extract_json_array(fl, "create:"));
                            } else if fl.starts_with("modify:") {
                                files_modify = Some(extract_json_array(fl, "modify:"));
                            } else if fl.starts_with("test:") {
                                files_test = Some(extract_json_array(fl, "test:"));
                            } else {
                                break;
                            }
                            i += 1;
                        }
                        continue;
                    } else if let Some(val) = strip_field(field, "depends_on:") {
                        depends_on = Some(normalize_array(&val));
                    } else if strip_field(field, "depends on：").is_some() || strip_field(field, "depends on:").is_some() {
                        let val = strip_field(field, "depends on：")
                            .or_else(|| strip_field(field, "depends on:"))
                            .unwrap_or_default();
                        depends_on = if val == "无" || val.is_empty() {
                            Some("[]".to_string())
                        } else {
                            Some(normalize_array(&val))
                        };
                    } else if field.starts_with("done_when:") || field.starts_with("Done when：") || field.starts_with("Done when:") {
                        let val = strip_field(field, "done_when:")
                            .or_else(|| strip_field(field, "Done when："))
                            .or_else(|| strip_field(field, "Done when:"))
                            .unwrap_or_default();
                        done_when = Some(if val.is_empty() {
                            "[]".to_string()
                        } else {
                            serde_json::to_string(&[&val]).unwrap_or_else(|_| format!("[\"{}\"]", val))
                        });
                    } else if strip_field(field, "details：").is_some() || strip_field(field, "details:").is_some() {
                        // 旧格式 details → 忽略（无法映射到 refs）
                    } else if strip_field(field, "parallel:").is_some() {
                        // 忽略 parallel 字段
                    }
                }
                i += 1;
            }

            tasks.push(TaskRecord {
                source_file: filename.to_string(),
                file_title: file_title.clone(),
                task_id,
                title,
                completed,
                priority,
                refs,
                files_create,
                files_modify,
                files_test,
                depends_on,
                complexity,
                done_when,
            });
        } else {
            i += 1;
        }
    }
    tasks
}

/// 从 issue 文件内容解析为 IssueRecord 列表
pub fn parse_issue_file(filename: &str, content: &str) -> Vec<IssueRecord> {
    let mut issues = Vec::new();
    let source_type = parse_frontmatter_field(content, "source");

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut issue_seq = 0u32;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("- [") {
            issue_seq += 1;
            let resolved = line.starts_with("- [x]");
            let after_bracket = if resolved { &line[6..] } else { &line[5..] };
            let after_bracket = after_bracket.trim();

            let (raw_id, title) = split_id_title(after_bracket);
            let issue_id = if raw_id == "UNKNOWN" {
                format!("I{:03}", issue_seq)
            } else {
                raw_id
            };

            let mut severity = None;
            let mut location = None;
            let mut description = None;
            let mut expected = None;
            let mut actual = None;
            let mut reproduce = None;
            let mut fix = None;

            i += 1;
            while i < lines.len() && !lines[i].starts_with("- [") {
                let sub = lines[i].trim();
                if sub.starts_with("- ") {
                    let field = &sub[2..];
                    if let Some(val) = strip_field(field, "severity:") {
                        severity = Some(val);
                    } else if let Some(val) = strip_field(field, "location:") {
                        location = Some(val);
                    } else if let Some(val) = strip_field(field, "description:") {
                        description = Some(val);
                    } else if let Some(val) = strip_field(field, "expected:") {
                        expected = Some(val);
                    } else if let Some(val) = strip_field(field, "actual:") {
                        actual = Some(val);
                    } else if let Some(val) = strip_field(field, "reproduce:") {
                        reproduce = Some(val);
                    } else if let Some(val) = strip_field(field, "fix:") {
                        fix = Some(val);
                    }
                } else if sub.starts_with("reproduce:") {
                    // 多行 reproduce（以 | 开头）
                    let mut repro_lines = Vec::new();
                    i += 1;
                    while i < lines.len() && !lines[i].trim().starts_with("- ") && !lines[i].starts_with("- [") {
                        repro_lines.push(lines[i].trim());
                        i += 1;
                    }
                    reproduce = Some(repro_lines.join("\n"));
                    continue;
                }
                i += 1;
            }

            issues.push(IssueRecord {
                source_file: filename.to_string(),
                source_type: source_type.clone(),
                issue_id,
                title,
                resolved,
                severity,
                location,
                description,
                expected,
                actual,
                reproduce,
                fix,
            });
        } else {
            i += 1;
        }
    }
    issues
}

/// 解析 changelog 内容，返回 (date, text) 对
pub fn parse_changelog(content: &str) -> Vec<(Option<String>, String)> {
    let mut entries = Vec::new();
    let mut current_date: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            let date_str = trimmed[3..].trim().to_string();
            current_date = Some(date_str);
        } else if trimmed.starts_with("- ") {
            entries.push((current_date.clone(), trimmed[2..].to_string()));
        }
    }
    entries
}

// ─── 迁移辅助 ───

/// 从归档目录名解析版本号和主题
pub fn parse_archive_dir_name(name: &str) -> Option<(String, String)> {
    // 格式：v<VERSION>-<TOPIC> 或 v<N>-<TOPIC>
    let name = name.strip_prefix('v')?;
    let dash_pos = name.find('-')?;
    let version = &name[..dash_pos];
    let topic = &name[dash_pos + 1..];
    Some((version.to_string(), topic.to_string()))
}

/// 迁移一个归档目录到 SQLite
pub fn migrate_archive_dir(
    conn: &Connection,
    dir: &Path,
    version: &str,
    topic: &str,
    branch: &str,
) -> Result<u32, DowError> {
    let mut count = 0u32;

    // 读取目录下的 task 文件
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (name.starts_with("done_task_") || name.starts_with("task_")) && name.ends_with(".md") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let tasks = parse_task_file(&name, &content);
                    for task in &tasks {
                        insert_task(conn, version, task)?;
                        count += 1;
                    }
                }
            }
        }
    }

    // 读取 issue 子目录
    let issue_dir = dir.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("closed_issue_") && name.ends_with(".md") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let issues = parse_issue_file(&name, &content);
                        for issue in &issues {
                            insert_issue(conn, version, issue)?;
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    // 读取文档
    for doc_type in &["PRD", "SPEC", "TEST"] {
        let doc_file = dir.join(format!("{}.md", doc_type));
        if doc_file.exists() {
            if let Ok(content) = fs::read_to_string(&doc_file) {
                insert_doc(conn, version, doc_type, &content)?;
                count += 1;
            }
        }
    }

    // 读取 CHANGELOG
    let changelog_file = dir.join("CHANGELOG.md");
    if changelog_file.exists() {
        if let Ok(content) = fs::read_to_string(&changelog_file) {
            let entries = parse_changelog(&content);
            for (order, (date, text)) in entries.iter().enumerate() {
                insert_changelog(conn, version, date.as_deref(), text, order as i32)?;
                count += 1;
            }
        }
    }

    // 插入 iteration 记录
    let released_at = extract_date_from_dir(dir);
    insert_iteration(conn, &IterationRecord {
        version: version.to_string(),
        topic: topic.to_string(),
        commit_type: None,
        branch: branch.to_string(),
        released_at,
        tag: format!("v{}", version),
        mode: None,
    })?;

    Ok(count)
}

// ─── 内部辅助函数 ───

fn parse_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return None;
    }
    for line in lines.iter().skip(1) {
        if line.trim() == "---" {
            break;
        }
        if let Some(val) = strip_field(line.trim(), &format!("{}:", field)) {
            return Some(val);
        }
    }
    None
}

fn strip_field(text: &str, prefix: &str) -> Option<String> {
    if text.starts_with(prefix) {
        Some(text[prefix.len()..].trim().to_string())
    } else {
        None
    }
}

fn split_id_title(text: &str) -> (String, String) {
    // 格式：`TASK-T001: title` 或 `T1：title` 或 `I1: title`
    if let Some(pos) = text.find(": ") {
        (text[..pos].to_string(), text[pos + 2..].to_string())
    } else if let Some(pos) = text.find("：") {
        (text[..pos].to_string(), text[pos + "：".len()..].to_string())
    } else if let Some(pos) = text.find(':') {
        (text[..pos].to_string(), text[pos + 1..].trim().to_string())
    } else {
        ("UNKNOWN".to_string(), text.to_string())
    }
}

fn normalize_task_id(raw_id: &str, seq: u32) -> String {
    // 如果已经是新格式 TASK-T001 就保留，否则统一转换
    if raw_id.starts_with("TASK-T") {
        raw_id.to_string()
    } else {
        format!("TASK-T{:03}", seq)
    }
}

fn normalize_array(val: &str) -> String {
    let val = val.trim();
    if val == "无" || val == "[]" || val.is_empty() {
        return "[]".to_string();
    }
    if val.starts_with('[') {
        return val.to_string();
    }
    // 逗号分隔转 JSON array
    let items: Vec<&str> = val.split(',').map(|s| s.trim()).collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

fn extract_json_array(line: &str, prefix: &str) -> String {
    let val = line.strip_prefix(prefix).unwrap_or(line).trim();
    if val.starts_with('[') {
        val.to_string()
    } else {
        normalize_array(val)
    }
}

fn extract_date_from_dir(dir: &Path) -> String {
    // 从 task 文件名提取日期
    // done_task_2026-05-26_1.md → 取 "2026-05-26"
    // task_2026-05-27_1.md → 取 "2026-05-27"
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".md").unwrap_or(&name);
            // 查找符合日期模式的部分（YYYY-MM-DD）
            for segment in stem.split('_') {
                if segment.len() == 10 && segment.chars().nth(4) == Some('-') && segment.chars().nth(7) == Some('-') {
                    return segment.to_string();
                }
            }
        }
    }
    // 回退：尝试从 CHANGELOG 提取
    let changelog = dir.join("CHANGELOG.md");
    if let Ok(content) = fs::read_to_string(&changelog) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("## ") {
                let date = trimmed[3..].trim();
                if date.len() == 10 {
                    return date.to_string();
                }
            }
        }
    }
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
