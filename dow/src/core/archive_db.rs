// dow/src/core/
// ├── archive_db.rs  -- SQLite archive storage (schema, read/write, parsing)
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::error::DowError;
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

// ─── Data Structures ───

pub struct IterationRecord {
    pub version: String,
    pub topic: String,
    pub commit_type: Option<String>,
    pub branch: String,
    pub released_at: String,
    pub tag: String,
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

// ─── Connection and Schema ───

/// Open archive database (always at .dev-doc/archive.db, shared across branches)
pub fn open_or_create(base: &Path) -> Result<Connection, DowError> {
    let db_path = base.join("archive.db");
    let conn = Connection::open(&db_path)
        .map_err(|e| DowError::new(format!("Failed to open archive.db: {}", e), 1))?;

    conn.execute_batch("PRAGMA journal_mode=DELETE;")
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    create_tables(&conn)?;
    Ok(conn)
}

/// Archive database base path (fixed at .dev-doc/, does not vary by branch)
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
            version TEXT NOT NULL,
            topic TEXT NOT NULL,
            commit_type TEXT,
            branch TEXT NOT NULL DEFAULT 'main',
            released_at TEXT NOT NULL,
            tag TEXT NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0
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
    .map_err(|e| DowError::new(format!("Failed to create tables: {}", e), 1))?;

    // Backwards compatibility: ensure revoked column exists
    conn.execute_batch("ALTER TABLE iterations ADD COLUMN revoked INTEGER NOT NULL DEFAULT 0;")
        .ok();

    // Backwards compatibility: remove version UNIQUE constraint (allow multiple records per version)
    let has_unique: bool = conn
        .query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='iterations'",
        [],
        |row| row.get::<_, String>(0),
        )
        .map(|sql| sql.contains("UNIQUE"))
        .unwrap_or(false);

    if has_unique {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS iterations_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version TEXT NOT NULL,
                topic TEXT NOT NULL,
                commit_type TEXT,
                branch TEXT NOT NULL DEFAULT 'main',
                released_at TEXT NOT NULL,
                tag TEXT NOT NULL,
                mode TEXT,
                revoked INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO iterations_new (id, version, topic, commit_type, branch, released_at, tag, mode, revoked)
                SELECT id, version, topic, commit_type, branch, released_at, tag, mode,
                    COALESCE(revoked, 0) FROM iterations;
            DROP TABLE iterations;
            ALTER TABLE iterations_new RENAME TO iterations;
        ").ok();
    }

    // Backwards compatibility: drop legacy `mode` column (no longer read or written).
    // Rebuild the table without it; preserves all other data and indexes.
    let has_mode: bool = conn
        .prepare("PRAGMA table_info(iterations)")
        .and_then(|mut stmt| {
            let names = stmt.query_map([], |r| r.get::<_, String>(1))?;
            for name in names {
                if name? == "mode" {
                    return Ok(true);
                }
            }
            Ok(false)
        })
        .unwrap_or(false);
    if has_mode {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS iterations_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version TEXT NOT NULL,
                topic TEXT NOT NULL,
                commit_type TEXT,
                branch TEXT NOT NULL DEFAULT 'main',
                released_at TEXT NOT NULL,
                tag TEXT NOT NULL,
                revoked INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO iterations_new (id, version, topic, commit_type, branch, released_at, tag, revoked)
                SELECT id, version, topic, commit_type, branch, released_at, tag, COALESCE(revoked, 0)
                FROM iterations;
            DROP TABLE iterations;
            ALTER TABLE iterations_new RENAME TO iterations;
            CREATE INDEX IF NOT EXISTS idx_iterations_version ON iterations(version);
        ").ok();
    }

    Ok(())
}

// ─── Write Operations ───

pub fn insert_iteration(conn: &Connection, rec: &IterationRecord) -> Result<(), DowError> {
    conn.execute(
        "INSERT INTO iterations (version, topic, commit_type, branch, released_at, tag, revoked)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
        params![
            rec.version,
            rec.topic,
            rec.commit_type,
            rec.branch,
            rec.released_at,
            rec.tag,
        ],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn insert_doc(
    conn: &Connection,
    version: &str,
    doc_type: &str,
    content: &str,
) -> Result<(), DowError> {
    let sql = match doc_type {
        "PRD" => "INSERT OR REPLACE INTO prd_docs (version, content) VALUES (?1, ?2)",
        "SPEC" => "INSERT OR REPLACE INTO spec_docs (version, content) VALUES (?1, ?2)",
        "TEST" => "INSERT OR REPLACE INTO test_docs (version, content) VALUES (?1, ?2)",
        "BRAINSTORM" => "INSERT OR REPLACE INTO brainstorm_docs (version, content) VALUES (?1, ?2)",
        _ => {
            return Err(DowError::new(
                format!("Unknown document type: {}", doc_type),
                1,
            ))
        }
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

pub fn insert_changelog(
    conn: &Connection,
    version: &str,
    date: Option<&str>,
    text: &str,
    order: i32,
) -> Result<(), DowError> {
    conn.execute(
        "INSERT INTO changelog_entries (version, entry_date, entry_text, sort_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![version, date, text, order],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

// ─── Query Operations ───

pub struct IterationSummary {
    pub version: String,
    pub topic: String,
    pub commit_type: Option<String>,
    pub branch: String,
    pub released_at: String,
    pub tag: String,
    pub task_count: i64,
    pub issue_count: i64,
}

pub fn list_iterations(
    conn: &Connection,
    branch: Option<&str>,
) -> Result<Vec<IterationSummary>, DowError> {
    let mut sql = String::from(
        "SELECT i.version, i.topic, i.commit_type, i.branch, i.released_at, i.tag,
         (SELECT COUNT(*) FROM tasks t WHERE t.version = i.version) as task_count,
         (SELECT COUNT(*) FROM issues s WHERE s.version = i.version) as issue_count
         FROM iterations i",
    );
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(b) = branch {
        sql.push_str(" WHERE i.branch = ?1");
        bind_values.push(b.to_string());
    }
    sql.push_str(" ORDER BY i.id ASC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt
        .query_map(params_slice.as_slice(), |row| {
        Ok(IterationSummary {
            version: row.get(0)?,
            topic: row.get(1)?,
            commit_type: row.get(2)?,
            branch: row.get(3)?,
            released_at: row.get(4)?,
            tag: row.get(5)?,
            task_count: row.get(6)?,
            issue_count: row.get(7)?,
        })
        })
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn get_doc(
    conn: &Connection,
    version: &str,
    doc_type: &str,
) -> Result<Option<String>, DowError> {
    let sql = match doc_type {
        "PRD" => "SELECT content FROM prd_docs WHERE version = ?1",
        "SPEC" => "SELECT content FROM spec_docs WHERE version = ?1",
        "TEST" => "SELECT content FROM test_docs WHERE version = ?1",
        "BRAINSTORM" => "SELECT content FROM brainstorm_docs WHERE version = ?1",
        _ => {
            return Err(DowError::new(
                format!("Unknown document type: {}", doc_type),
                1,
            ))
        }
    };
    let result = conn
        .query_row(sql, params![version], |row| row.get::<_, String>(0))
        .ok();
    Ok(result)
}

pub fn query_tasks(
    conn: &Connection,
    version: Option<&str>,
    priority: Option<&str>,
) -> Result<Vec<TaskRecord>, DowError> {
    let mut sql = String::from(
        "SELECT source_file, file_title, task_id, title, completed, priority, refs,
         files_create, files_modify, files_test, depends_on, complexity, done_when
         FROM tasks WHERE 1=1",
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

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt
        .query_map(params_slice.as_slice(), |row| {
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
        })
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn query_issues(
    conn: &Connection,
    version: Option<&str>,
    severity: Option<&str>,
) -> Result<Vec<IssueRecord>, DowError> {
    let mut sql = String::from(
        "SELECT source_file, source_type, issue_id, title, resolved, severity,
         location, description, expected, actual, reproduce, fix
         FROM issues WHERE 1=1",
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

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let params_slice: Vec<&dyn rusqlite::ToSql> = bind_values
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt
        .query_map(params_slice.as_slice(), |row| {
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
        })
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

pub fn query_changelog(
    conn: &Connection,
    version: &str,
) -> Result<Vec<(Option<String>, String)>, DowError> {
    let mut stmt = conn.prepare(
        "SELECT entry_date, entry_text FROM changelog_entries WHERE version = ?1 ORDER BY sort_order ASC"
    ).map_err(|e| DowError::new(e.to_string(), 1))?;

    let rows = stmt
        .query_map(params![version], |row| {
        Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DowError::new(e.to_string(), 1))?);
    }
    Ok(results)
}

/// Find the latest iteration id where version=X and revoked=0
pub fn find_active_iteration(conn: &Connection, version: &str) -> Result<Option<i64>, DowError> {
    let result = conn.query_row(
        "SELECT id FROM iterations WHERE version = ?1 AND revoked = 0 ORDER BY id DESC LIMIT 1",
        params![version],
        |row| row.get::<_, i64>(0),
    );
    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DowError::new(e.to_string(), 1)),
    }
}

/// Mark specified iteration id as revoked
pub fn mark_iteration_revoked(conn: &Connection, iteration_id: i64) -> Result<(), DowError> {
    conn.execute_batch("ALTER TABLE iterations ADD COLUMN revoked INTEGER NOT NULL DEFAULT 0;")
        .ok(); // Backwards compatibility

    conn.execute(
        "UPDATE iterations SET revoked = 1 WHERE id = ?1",
        params![iteration_id],
    )
    .map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok(())
}

pub fn get_stats(conn: &Connection) -> Result<serde_json::Value, DowError> {
    let iter_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM iterations", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    let task_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    let issue_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issues", [], |r| r.get(0))
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    Ok(serde_json::json!({
        "iterations": iter_count,
        "tasks": task_count,
        "issues": issue_count,
    }))
}

// ─── Parsing Logic ───

/// Parse task file content into TaskRecord list (compatible with old/new formats, old formats are converted)
pub fn parse_task_file(filename: &str, content: &str) -> Vec<TaskRecord> {
    let mut tasks = Vec::new();
    let file_title = parse_frontmatter_field(content, "title");

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut task_seq = 0;

    while i < lines.len() {
        let line = lines[i];
        // Match task item line: `- [x] TASK-T001: xxx` or `- [x] T1：xxx`
        if line.starts_with("- [") {
            task_seq += 1;
            let completed = line.starts_with("- [x]");
            let after_bracket = if completed { &line[6..] } else { &line[5..] };
            let after_bracket = after_bracket.trim();

            // Parse task_id and title
            let (raw_id, title) = split_id_title(after_bracket);
            let task_id = normalize_task_id(&raw_id, task_seq);

            // Collect subfields
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
                        // Old format → convert to priority
                        priority = Some(val);
                    } else if let Some(val) = strip_field(field, "refs:") {
                        refs = Some(val);
                    } else if let Some(val) = strip_field(field, "complexity:") {
                        complexity = Some(val);
                    } else if field.starts_with("files:") {
                        // Parse files subblock
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
                    } else if strip_field(field, "depends on：").is_some()
                        || strip_field(field, "depends on:").is_some()
                    {
                        let val = strip_field(field, "depends on：")
                            .or_else(|| strip_field(field, "depends on:"))
                            .unwrap_or_default();
                        depends_on = if val == "none" || val.is_empty() {
                            Some("[]".to_string())
                        } else {
                            Some(normalize_array(&val))
                        };
                    } else if field.starts_with("done_when:")
                        || field.starts_with("Done when：")
                        || field.starts_with("Done when:")
                    {
                        let val = strip_field(field, "done_when:")
                            .or_else(|| strip_field(field, "Done when："))
                            .or_else(|| strip_field(field, "Done when:"))
                            .unwrap_or_default();
                        done_when = Some(if val.is_empty() {
                            "[]".to_string()
                        } else {
                            serde_json::to_string(&[&val])
                                .unwrap_or_else(|_| format!("[\"{}\"]", val))
                        });
                    } else if strip_field(field, "details：").is_some()
                        || strip_field(field, "details:").is_some()
                    {
                        // Old format details → ignore (cannot map to refs)
                    } else if strip_field(field, "parallel:").is_some() {
                        // Ignore parallel field
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

/// Parse issue file content into IssueRecord list
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
                    // Multi-line reproduce (starts with |)
                    let mut repro_lines = Vec::new();
                    i += 1;
                    while i < lines.len()
                        && !lines[i].trim().starts_with("- ")
                        && !lines[i].starts_with("- [")
                    {
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

/// Parse changelog content, return (date, text) pairs
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

// ─── Migration Helpers ───

/// Parse version and topic from archive directory name
pub fn parse_archive_dir_name(name: &str) -> Option<(String, String)> {
    // Format: v<VERSION>-<TOPIC> or v<N>-<TOPIC>
    let name = name.strip_prefix('v')?;
    let dash_pos = name.find('-')?;
    let version = &name[..dash_pos];
    let topic = &name[dash_pos + 1..];
    Some((version.to_string(), topic.to_string()))
}

/// Migrate an archive directory to SQLite
pub fn migrate_archive_dir(
    conn: &Connection,
    dir: &Path,
    version: &str,
    topic: &str,
    branch: &str,
) -> Result<u32, DowError> {
    let mut count = 0u32;

    // Read task files from directory
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (name.starts_with("done_task_") || name.starts_with("task_"))
                && name.ends_with(".md")
            {
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

    // Read issue subdirectory
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

    // Read documents
    for doc_type in &["PRD", "SPEC", "TEST"] {
        let doc_file = dir.join(format!("{}.md", doc_type));
        if doc_file.exists() {
            if let Ok(content) = fs::read_to_string(&doc_file) {
                insert_doc(conn, version, doc_type, &content)?;
                count += 1;
            }
        }
    }

    // Read CHANGELOG
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

    // Insert iteration record
    let released_at = extract_date_from_dir(dir);
    insert_iteration(
        conn,
        &IterationRecord {
        version: version.to_string(),
        topic: topic.to_string(),
        commit_type: None,
        branch: branch.to_string(),
        released_at,
        tag: format!("v{}", version),
        },
    )?;

    Ok(count)
}

// ─── Internal Helper Functions ───

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
    // Format: `TASK-T001: title` or `T1：title` or `I1: title`
    if let Some(pos) = text.find(": ") {
        (text[..pos].to_string(), text[pos + 2..].to_string())
    } else if let Some(pos) = text.find("：") {
        (
            text[..pos].to_string(),
            text[pos + "：".len()..].to_string(),
        )
    } else if let Some(pos) = text.find(':') {
        (text[..pos].to_string(), text[pos + 1..].trim().to_string())
    } else {
        ("UNKNOWN".to_string(), text.to_string())
    }
}

fn normalize_task_id(raw_id: &str, seq: u32) -> String {
    // If already in new format TASK-T001, keep it; otherwise convert uniformly
    if raw_id.starts_with("TASK-T") {
        raw_id.to_string()
    } else {
        format!("TASK-T{:03}", seq)
    }
}

fn normalize_array(val: &str) -> String {
    let val = val.trim();
    if val == "none" || val == "[]" || val.is_empty() {
        return "[]".to_string();
    }
    if val.starts_with('[') {
        return val.to_string();
    }
    // Convert comma-separated to JSON array
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
    // Extract date from task filename
    // done_task_2026-05-26_1.md → extract "2026-05-26"
    // task_2026-05-27_1.md → extract "2026-05-27"
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".md").unwrap_or(&name);
            // Find segment matching date pattern (YYYY-MM-DD)
            for segment in stem.split('_') {
                if segment.len() == 10
                    && segment.chars().nth(4) == Some('-')
                    && segment.chars().nth(7) == Some('-')
                {
                    return segment.to_string();
                }
            }
        }
    }
    // Fallback: try extracting from CHANGELOG
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: an archive DB with the legacy `mode` column must be migrated
    /// on open — `mode` dropped, all other columns and row data preserved.
    #[test]
    fn test_migrate_drops_mode_column_preserving_data() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("archive.db");

        // Old schema (with mode) + one row carrying a mode value.
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE iterations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    version TEXT NOT NULL,
                    topic TEXT NOT NULL,
                    commit_type TEXT,
                    branch TEXT NOT NULL DEFAULT 'main',
                    released_at TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    mode TEXT,
                    revoked INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO iterations (version, topic, commit_type, branch, released_at, tag, mode, revoked)
                    VALUES ('0.9.0', 'legacy-release', 'fix', 'main', '2026-01-01', 'v0.9.0', 'fast', 0);
                CREATE INDEX idx_iterations_version ON iterations(version);",
            ).unwrap();
        }

        // Trigger migration.
        let conn = open_or_create(dir.path()).unwrap();

        // mode column must be gone.
        let has_mode: bool = {
            let mut stmt = conn.prepare("PRAGMA table_info(iterations)").unwrap();
            let names = stmt.query_map([], |r| r.get::<_, String>(1)).unwrap();
            let mut found = false;
            for name in names {
                if name.unwrap() == "mode" {
                    found = true;
                }
            }
            found
        };
        assert!(!has_mode, "mode column should be dropped after migration");

        // All other data preserved.
        let row: (String, String, Option<String>, String, i64) = conn
            .query_row(
                "SELECT version, topic, commit_type, tag, revoked FROM iterations WHERE version = '0.9.0'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(row.0, "0.9.0");
        assert_eq!(row.1, "legacy-release");
        assert_eq!(row.2, Some("fix".to_string()));
        assert_eq!(row.3, "v0.9.0");
        assert_eq!(row.4, 0);
    }
}
