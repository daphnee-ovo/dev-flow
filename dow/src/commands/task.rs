// FrameworkTree
// task.rs
// ├── struct TaskInput
// ├── struct TaskCreatePayload
// ├── struct TaskFilesInput
// ├── struct TaskListItem
// ├── struct TaskDetail
// ├── struct TaskFiles
// ├── struct ReopenImpact
// ├── expand_braces()
// ├── expand_file_list()
// ├── apply_incremental()
// ├── run()
// ├── create()
// ├── resolve_create_input()
// ├── has_any_task_create_flag()
// ├── read_stdin_if_available()
// ├── parse_task_files_arg()
// ├── normalize_task_files()
// ├── validate_task_file_scope()
// ├── task_input_from_payload()
// ├── split_comma()
// ├── validate_complexity()
// ├── scan_next_id()
// ├── all_task_files_including_done()
// ├── extract_task_id()
// ├── parse_task_num()
// ├── find_or_create_batch_file()
// ├── format_task_entry()
// ├── format_string_list()
// ├── count_tasks_in_content()
// ├── update_frontmatter_nums()
// ├── struct TaskUpdateInput
// ├── update()
// ├── resolve_update_input()
// ├── has_any_task_update()
// ├── replace_task_entry_in_content()
// ├── struct RemoveImpact
// ├── struct RenumberEntry
// ├── remove()
// ├── remove_task_entry()
// ├── purge_id_from_depends_on()
// ├── find_owning_task_id()
// ├── generate_trm_token()
// ├── list()
// ├── parse_all_tasks_in_file()
// ├── show()
// ├── parse_task_detail()
// ├── parse_inline_list()
// ├── print_task_detail_human()
// ├── done_multi()
// ├── done_single()
// ├── atomic_write()
// ├── reopen()
// ├── generate_tro_token()
// ├── schema()
// ├── resolve_task_dir()
// └── get_all_task_details()

// dow/src/commands/
// ├── task.rs  -- dow task (task resource management)
//    ├── run()
//    ├── create()
//    ├── update()
//    ├── list()
//    ├── show()
//    ├── done_multi() / done_single()
//    ├── reopen()
//    ├── schema()
//
// Related Docs:
// - [CLAUDE.md - Task File Format](../../../CLAUDE.md)

use crate::cli::{
    TaskCommands, TaskCreateArgs, TaskListArgs, TaskRemoveArgs, TaskReopenArgs, TaskUpdateArgs,
};
use crate::commands::test_runner;
use crate::core::{claim, doc_root, item_id, task_store};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct TaskInput {
    title: String,
    r#type: String,
    priority: String,
    refs: String,
    files_modify: Vec<String>,
    files_create: Vec<String>,
    files_test: Vec<String>,
    depends_on: Vec<String>,
    parallel: bool,
    complexity: String,
    done_when: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskCreatePayload {
    title: String,
    #[serde(rename = "type")]
    task_type: String,
    priority: String,
    refs: String,
    files: TaskFilesInput,
    depends_on: Vec<String>,
    parallel: bool,
    complexity: String,
    done_when: Vec<String>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct TaskFilesInput {
    create: Option<Vec<String>>,
    modify: Option<Vec<String>>,
    test: Option<Vec<String>>,
}

#[derive(Serialize)]
struct TaskListItem {
    id: String,
    title: String,
    r#type: String,
    priority: String,
    status: String,
    file: String,
}

#[derive(Serialize)]
pub(crate) struct TaskDetail {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) r#type: String,
    pub(crate) priority: String,
    pub(crate) status: String,
    pub(crate) refs: String,
    pub(crate) files: TaskFiles,
    pub(crate) depends_on: Vec<String>,
    pub(crate) parallel: bool,
    pub(crate) complexity: String,
    pub(crate) done_when: Vec<String>,
    pub(crate) file: String,
}

#[derive(Serialize)]
pub(crate) struct TaskFiles {
    pub(crate) create: Vec<String>,
    pub(crate) modify: Vec<String>,
    pub(crate) test: Vec<String>,
}

#[derive(Serialize)]
struct ReopenImpact {
    id: String,
    title: String,
    file: String,
    confirm_token: String,
    message: String,
}

// ─── Brace Expansion ─────────────────────────────────────────────────────────

pub(crate) fn expand_braces(input: &str) -> Vec<String> {
    let Some(open) = input.find('{') else {
        return vec![input.to_string()];
    };
    let Some(close) = input[open..].find('}') else {
        return vec![input.to_string()];
    };
    let close = open + close;
    let prefix = &input[..open];
    let suffix = &input[close + 1..];
    let inner = &input[open + 1..close];

    inner
        .split(',')
        .flat_map(|part| expand_braces(&format!("{}{}{}", prefix, part.trim(), suffix)))
        .collect()
}

pub(crate) fn expand_file_list(files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .flat_map(|f| expand_braces(&f))
        .filter(|f| !f.is_empty())
        .collect()
}

/// Apply incremental update to an array field.
/// If any item in `input` starts with '+' or '-', treat as incremental:
///   +item → append to existing, -item → remove from existing.
/// If no items have prefix → full replacement (backward compatible).
pub(crate) fn apply_incremental(input: Vec<String>, existing: Vec<String>) -> Vec<String> {
    let is_incremental = input
        .iter()
        .any(|s| s.starts_with('+') || s.starts_with('-'));
    if !is_incremental {
        return input;
    }

    let mut result = existing;
    for item in &input {
        if let Some(val) = item.strip_prefix('+') {
            let val = val.to_string();
            if !val.is_empty() && !result.contains(&val) {
                result.push(val);
            }
        } else if let Some(val) = item.strip_prefix('-') {
            result.retain(|x| x != val);
        }
        // items without prefix in incremental mode are ignored
    }
    result
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

pub fn run(command: TaskCommands, human: bool) -> Result<i32, DowError> {
    match command {
        TaskCommands::Create(args) => create(args, human),
        TaskCommands::Update(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            update(args)
        }
        TaskCommands::Remove(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            remove(args, human)
        }
        TaskCommands::List(args) => list(args, human),
        TaskCommands::Show { id } => show(&item_id::normalize_full(&id), human),
        TaskCommands::Done { ids } => {
            let ids: Vec<String> = ids.iter().map(|id| item_id::normalize_full(id)).collect();
            done_multi(&ids)
        }
        TaskCommands::Reopen(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            reopen(args, human)
        }
        TaskCommands::Schema => schema(human),
    }
}

// ─── Create ──────────────────────────────────────────────────────────────────

fn create(args: TaskCreateArgs, _human: bool) -> Result<i32, DowError> {
    let tasks = resolve_create_input(args)?;
    if tasks.is_empty() {
        return Err(DowError::new(
            "no task input provided (use --title or pipe JSON to stdin)",
            2,
        ));
    }

    for task in &tasks {
        validate_complexity(&task.complexity)?;
    }

    let task_dir = resolve_task_dir()?;
    fs::create_dir_all(&task_dir)
        .map_err(|e| DowError::new(format!("cannot create task directory: {}", e), 1))?;

    let next_id = scan_next_id(&task_dir);
    let today = Local::now().format("%Y-%m-%d").to_string();
    let batch_file = find_or_create_batch_file(&task_dir, &today)?;

    let mut content = fs::read_to_string(&batch_file)
        .map_err(|e| DowError::new(format!("cannot read task file: {}", e), 1))?;

    // Update nums in frontmatter
    let existing_count = count_tasks_in_content(&content);
    let new_total = existing_count + tasks.len();

    let mut created_ids = Vec::new();
    for (i, task) in tasks.iter().enumerate() {
        let id_num = next_id + i;
        let id_str = format!("TASK-T{:03}", id_num);
        let entry = format_task_entry(&id_str, task);
        content.push_str(&entry);
        created_ids.push(id_str);
    }

    // Update frontmatter nums
    content = update_frontmatter_nums(&content, new_total);

    fs::write(&batch_file, &content)
        .map_err(|e| DowError::new(format!("cannot write task file: {}", e), 1))?;

    for id in &created_ids {
        println!("{}", id);
    }

    Ok(0)
}

fn resolve_create_input(args: TaskCreateArgs) -> Result<Vec<TaskInput>, DowError> {
    let has_flags = has_any_task_create_flag(&args);
    let stdin_data = read_stdin_if_available();
    if has_flags && stdin_data.is_some() {
        return Err(DowError::new(
            "cannot combine task CLI options with stdin JSON; use one input source",
            2,
        ));
    }

    if let Some(stdin_data) = stdin_data {
        let trimmed = stdin_data.trim();
        if trimmed.starts_with('[') {
            let tasks: Vec<TaskCreatePayload> = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON array from stdin: {}", e), 2))?;
            return tasks.into_iter().map(task_input_from_payload).collect();
        } else if trimmed.starts_with('{') {
            let task: TaskCreatePayload = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON object from stdin: {}", e), 2))?;
            return Ok(vec![task_input_from_payload(task)?]);
        }
        return Err(DowError::new(
            "stdin input must be a JSON object or array",
            2,
        ));
    }

    // Use flags — all fields required
    let title = match args.title {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };
    let task_type = args
        .task_type
        .ok_or_else(|| DowError::new("--task-type is required", 2))?;
    let priority = args
        .priority
        .ok_or_else(|| DowError::new("--priority is required", 2))?;
    let refs = args
        .refs
        .ok_or_else(|| DowError::new("--refs is required", 2))?;
    let file = args
        .file
        .ok_or_else(|| DowError::new("--file is required", 2))?;
    let depends_on = args
        .depends_on
        .ok_or_else(|| DowError::new("--depends-on is required", 2))?;
    let complexity = args
        .complexity
        .ok_or_else(|| DowError::new("--complexity is required", 2))?;
    let done_when = args
        .done_when
        .ok_or_else(|| DowError::new("--done-when is required", 2))?;

    let task = TaskCreatePayload {
        title,
        task_type,
        priority,
        refs,
        files: parse_task_files_arg(&file)?,
        depends_on: split_comma(&Some(depends_on)),
        parallel: args.parallel,
        complexity,
        done_when: split_comma(&Some(done_when)),
    };

    Ok(vec![task_input_from_payload(task)?])
}

fn has_any_task_create_flag(args: &TaskCreateArgs) -> bool {
    args.title.is_some()
        || args.task_type.is_some()
        || args.priority.is_some()
        || args.refs.is_some()
        || args.file.is_some()
        || args.depends_on.is_some()
        || args.complexity.is_some()
        || args.done_when.is_some()
        || args.parallel
}

fn read_stdin_if_available() -> Option<String> {
    use std::io::IsTerminal;

    // If stdin is a terminal (interactive), no piped data
    if io::stdin().is_terminal() {
        return None;
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    if buf.trim().is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn parse_task_files_arg(value: &str) -> Result<TaskFilesInput, DowError> {
    serde_json::from_str(value)
        .map_err(|e| DowError::new(format!("invalid --file JSON object: {}", e), 2))
}

fn normalize_task_files(files: &mut TaskFilesInput) {
    if let Some(values) = files.create.take() {
        files.create = Some(expand_file_list(values));
    }
    if let Some(values) = files.modify.take() {
        files.modify = Some(expand_file_list(values));
    }
    if let Some(values) = files.test.take() {
        files.test = Some(expand_file_list(values));
    }
}

fn validate_task_file_scope(files: &TaskFilesInput, operation: &str) -> Result<(), DowError> {
    let has_create = files
        .create
        .as_ref()
        .is_some_and(|values| !values.is_empty());
    let has_modify = files
        .modify
        .as_ref()
        .is_some_and(|values| !values.is_empty());
    if has_create || has_modify {
        return Ok(());
    }

    Err(DowError::new(
        format!(
            "{} requires at least one non-empty files.create or files.modify list",
            operation
        ),
        2,
    ))
}

fn task_input_from_payload(mut payload: TaskCreatePayload) -> Result<TaskInput, DowError> {
    normalize_task_files(&mut payload.files);
    validate_task_file_scope(&payload.files, "task create")?;

    Ok(TaskInput {
        title: payload.title,
        r#type: payload.task_type,
        priority: payload.priority,
        refs: payload.refs,
        files_modify: payload.files.modify.unwrap_or_default(),
        files_create: payload.files.create.unwrap_or_default(),
        files_test: payload.files.test.unwrap_or_default(),
        depends_on: payload.depends_on,
        parallel: payload.parallel,
        complexity: payload.complexity,
        done_when: payload.done_when,
    })
}

fn split_comma(opt: &Option<String>) -> Vec<String> {
    match opt {
        Some(s) if !s.is_empty() => s.split(',').map(|x| x.trim().to_string()).collect(),
        _ => Vec::new(),
    }
}

fn validate_complexity(complexity: &str) -> Result<(), DowError> {
    if ["S", "M", "L"].contains(&complexity) {
        return Ok(());
    }
    Err(DowError::new(
        format!(
            "invalid complexity '{}', valid: S/M/L; split oversized work into multiple Tasks",
            complexity
        ),
        2,
    ))
}

fn scan_next_id(task_dir: &Path) -> usize {
    let mut max_id: usize = 0;

    let all_files = all_task_files_including_done(task_dir);
    for path in all_files {
        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(id) = extract_task_id(line) {
                    if let Some(num) = parse_task_num(&id) {
                        if num > max_id {
                            max_id = num;
                        }
                    }
                }
            }
        }
    }

    max_id + 1
}

pub(crate) fn all_task_files_including_done(task_dir: &Path) -> Vec<PathBuf> {
    if !task_dir.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(task_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (name.starts_with("task_") || name.starts_with("done_task_")) && name.ends_with(".md")
        })
        .map(|e| e.path())
        .collect()
}

fn extract_task_id(line: &str) -> Option<String> {
    let parsed = item_id::extract_from_line(line)?;
    if parsed.kind == item_id::ItemKind::Task {
        Some(parsed.full())
    } else {
        None
    }
}

fn parse_task_num(id: &str) -> Option<usize> {
    let parsed = item_id::parse(id)?;
    Some(parsed.num() as usize)
}

fn find_or_create_batch_file(task_dir: &Path, today: &str) -> Result<PathBuf, DowError> {
    let mut max_seq = 0u32;
    let prefix_active = format!("task_{}_", today);
    let prefix_done = format!("done_task_{}_", today);

    if let Ok(entries) = fs::read_dir(task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            let seq_str = if name.starts_with(&prefix_active) {
                Some(&name[prefix_active.len()..name.len() - 3])
            } else if name.starts_with(&prefix_done) {
                Some(&name[prefix_done.len()..name.len() - 3])
            } else {
                None
            };
            if let Some(s) = seq_str {
                if let Ok(seq) = s.parse::<u32>() {
                    if seq > max_seq {
                        max_seq = seq;
                    }
                }
            }
        }
    }

    // Try appending to existing active batch file with highest seq
    if max_seq > 0 {
        let path = task_dir.join(format!("task_{}_{}.md", today, max_seq));
        if path.exists() {
            return Ok(path);
        }
    }

    // Create new batch file (seq must be above all existing active + done)
    let seq = max_seq + 1;
    let path = task_dir.join(format!("task_{}_{}.md", today, seq));
    let frontmatter = format!("---\ntitle: TASK - batch {}\nnums: 0\n---\n\n", today);
    fs::write(&path, &frontmatter)
        .map_err(|e| DowError::new(format!("cannot create batch file: {}", e), 1))?;
    Ok(path)
}

fn format_task_entry(id: &str, task: &TaskInput) -> String {
    let mut lines = Vec::new();
    lines.push(format!("- [ ] {}: {}", id, task.title));
    lines.push(format!("  - type: {}", task.r#type));
    lines.push(format!("  - priority: {}", task.priority));
    if !task.refs.is_empty() {
        lines.push(format!("  - refs: {}", task.refs));
    }
    lines.push("  - files:".to_string());
    lines.push(format!(
        "      create: [{}]",
        format_string_list(&task.files_create)
    ));
    lines.push(format!(
        "      modify: [{}]",
        format_string_list(&task.files_modify)
    ));
    lines.push(format!(
        "      test: [{}]",
        format_string_list(&task.files_test)
    ));
    lines.push(format!(
        "  - depends_on: [{}]",
        format_string_list(&task.depends_on)
    ));
    lines.push(format!("  - parallel: {}", task.parallel));
    lines.push(format!("  - complexity: {}", task.complexity));
    lines.push("  - done_when:".to_string());
    if task.done_when.is_empty() {
        lines.push("      - task completed".to_string());
    } else {
        for criterion in &task.done_when {
            lines.push(format!("      - {}", criterion));
        }
    }
    lines.push(String::new()); // trailing newline between tasks
    lines.join("\n")
}

fn format_string_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    items
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ")
}

fn count_tasks_in_content(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- [ ] TASK-T") || t.starts_with("- [x] TASK-T")
        })
        .count()
}

fn update_frontmatter_nums(content: &str, new_count: usize) -> String {
    let mut result = String::new();
    for line in content.lines() {
        if line.starts_with("nums:") {
            result.push_str(&format!("nums: {}", new_count));
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

// ─── Update ─────────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct TaskUpdateInput {
    title: Option<String>,
    #[serde(rename = "type")]
    task_type: Option<String>,
    priority: Option<String>,
    refs: Option<String>,
    files: Option<TaskFilesInput>,
    depends_on: Option<Vec<String>>,
    parallel: Option<bool>,
    complexity: Option<String>,
    done_when: Option<Vec<String>>,
}

fn update(args: TaskUpdateArgs) -> Result<i32, DowError> {
    let id = args.id.clone();
    let mut input = resolve_update_input(args)?;

    if !has_any_task_update(&input) {
        return Err(DowError::new(
            "no fields to update (provide at least one --field)",
            2,
        ));
    }

    if let Some(files) = input.files.as_mut() {
        normalize_task_files(files);
        validate_task_file_scope(files, "task update")?;
    }

    // Validate enum fields if provided
    if let Some(ref t) = input.task_type {
        let valid = ["feat", "fix", "refactor", "docs", "perf", "test", "style"];
        if !valid.contains(&t.as_str()) {
            return Err(DowError::new(
                format!(
                    "invalid type '{}', valid: feat/fix/refactor/docs/perf/test/style",
                    t
                ),
                2,
            ));
        }
    }
    if let Some(ref p) = input.priority {
        let valid = ["P0", "P1", "P2"];
        if !valid.contains(&p.as_str()) {
            return Err(DowError::new(
                format!("invalid priority '{}', valid: P0/P1/P2", p),
                2,
            ));
        }
    }
    if let Some(ref c) = input.complexity {
        validate_complexity(c)?;
    }

    let task_dir = resolve_task_dir()?;
    let all_files = all_task_files_including_done(&task_dir);

    // Find the task
    for path in &all_files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if let Ok(content) = fs::read_to_string(path) {
            if let Some(detail) = parse_task_detail(&content, &id, &filename) {
                if detail.status == "done" {
                    return Err(DowError::new(
                        format!("cannot update completed task {} (use 'reopen' first)", id),
                        1,
                    ));
                }

                // Merge (array fields use incremental logic)
                let new_files_modify =
                    match input.files.as_ref().and_then(|files| files.modify.clone()) {
                        Some(values) => apply_incremental(values, detail.files.modify),
                        None => detail.files.modify,
                    };
                let new_files_create =
                    match input.files.as_ref().and_then(|files| files.create.clone()) {
                        Some(values) => apply_incremental(values, detail.files.create),
                        None => detail.files.create,
                    };
                if input.files.is_some()
                    && new_files_modify.is_empty()
                    && new_files_create.is_empty()
                {
                    return Err(DowError::new(
                        "task update cannot remove the last files.create/files.modify path",
                        2,
                    ));
                }

                let merged = TaskInput {
                    title: input.title.clone().unwrap_or(detail.title),
                    r#type: input.task_type.clone().unwrap_or(detail.r#type),
                    priority: input.priority.clone().unwrap_or(detail.priority),
                    refs: input.refs.clone().unwrap_or(detail.refs),
                    files_modify: new_files_modify,
                    files_create: new_files_create,
                    files_test: match input.files.as_ref().and_then(|files| files.test.clone()) {
                        Some(values) => apply_incremental(values, detail.files.test),
                        None => detail.files.test,
                    },
                    depends_on: match input.depends_on.clone() {
                        Some(values) => apply_incremental(values, detail.depends_on),
                        None => detail.depends_on,
                    },
                    parallel: input.parallel.unwrap_or(detail.parallel),
                    complexity: input.complexity.clone().unwrap_or(detail.complexity),
                    done_when: match input.done_when.clone() {
                        Some(values) => apply_incremental(values, detail.done_when),
                        None => detail.done_when,
                    },
                };

                let new_content = replace_task_entry_in_content(&content, &id, &merged);
                fs::write(path, &new_content)
                    .map_err(|e| DowError::new(format!("cannot write task file: {}", e), 1))?;
                return Ok(0);
            }
        }
    }

    Err(DowError::new(format!("task {} not found", id), 1))
}

fn resolve_update_input(args: TaskUpdateArgs) -> Result<TaskUpdateInput, DowError> {
    let has_flags = args.title.is_some()
        || args.task_type.is_some()
        || args.priority.is_some()
        || args.refs.is_some()
        || args.file.is_some()
        || args.depends_on.is_some()
        || args.parallel.is_some()
        || args.complexity.is_some()
        || args.done_when.is_some();
    let stdin_data = read_stdin_if_available();
    if has_flags && stdin_data.is_some() {
        return Err(DowError::new(
            "cannot combine task update CLI options with stdin JSON; use one input source",
            2,
        ));
    }
    if let Some(stdin_data) = stdin_data {
        let trimmed = stdin_data.trim();
        if trimmed.starts_with('{') {
            let input: TaskUpdateInput = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON from stdin: {}", e), 2))?;
            return Ok(input);
        }
        return Err(DowError::new(
            "task update stdin input must be a JSON object",
            2,
        ));
    }

    Ok(TaskUpdateInput {
        title: args.title,
        task_type: args.task_type,
        priority: args.priority,
        refs: args.refs,
        files: args
            .file
            .map(|value| parse_task_files_arg(&value))
            .transpose()?,
        depends_on: args.depends_on.map(|s| split_comma(&Some(s))),
        parallel: args.parallel,
        complexity: args.complexity,
        done_when: args.done_when.map(|s| split_comma(&Some(s))),
    })
}

fn has_any_task_update(input: &TaskUpdateInput) -> bool {
    input.title.is_some()
        || input.task_type.is_some()
        || input.priority.is_some()
        || input.refs.is_some()
        || input.files.is_some()
        || input.depends_on.is_some()
        || input.parallel.is_some()
        || input.complexity.is_some()
        || input.done_when.is_some()
}

fn replace_task_entry_in_content(content: &str, target_id: &str, task: &TaskInput) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(id) = extract_task_id(line) {
            if id == target_id {
                // Write the updated entry
                let entry = format_task_entry(&id, task);
                result.push(entry.trim_end().to_string());
                // Skip old sub-fields
                i += 1;
                while i < lines.len() {
                    let sub = lines[i].trim();
                    if sub.starts_with("- [ ]") || sub.starts_with("- [x]") || sub.is_empty() {
                        break;
                    }
                    i += 1;
                }
                continue;
            }
        }
        result.push(line.to_string());
        i += 1;
    }

    let mut out = result.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ─── Remove ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RemoveImpact {
    id: String,
    title: String,
    renumber: Vec<RenumberEntry>,
    depends_on_updates: Vec<String>,
    confirm_token: String,
    command: String,
}

#[derive(Serialize)]
struct RenumberEntry {
    from: String,
    to: String,
}

fn remove(args: TaskRemoveArgs, human: bool) -> Result<i32, DowError> {
    let id = &args.id;
    let task_dir = resolve_task_dir()?;
    let all_files = task_store::iter_task_files(&task_dir);

    // Find target task — must be pending
    let mut found_file: Option<PathBuf> = None;
    let mut found_title = String::new();
    let mut target_num: Option<usize> = None;

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [ ] {}:", id)) {
                found_file = Some(path.clone());
                for line in content.lines() {
                    if line.contains(&format!("- [ ] {}:", id)) {
                        let after_id = line.split(':').skip(1).collect::<Vec<_>>().join(":");
                        found_title = after_id.trim().to_string();
                        break;
                    }
                }
                target_num = parse_task_num(id);
                break;
            }
        }
    }

    let file_path = match found_file {
        Some(p) => p,
        None => return Err(DowError::new(format!("pending task {} not found", id), 1)),
    };
    let target_num = target_num.unwrap_or(0);

    // Collect all task IDs with higher numbers (they'll be renumbered)
    let all_files_full = all_task_files_including_done(&task_dir);
    let mut higher_ids: Vec<usize> = Vec::new();
    let mut deps_affected: Vec<String> = Vec::new();

    for path in &all_files_full {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(tid) = extract_task_id(line) {
                    if let Some(num) = parse_task_num(&tid) {
                        if num > target_num {
                            higher_ids.push(num);
                        }
                    }
                }
                // Check depends_on references to target
                if line.contains("depends_on:") && line.contains(id) {
                    if let Some(tid) = find_owning_task_id(&content, line) {
                        if !deps_affected.contains(&tid) {
                            deps_affected.push(tid);
                        }
                    }
                }
            }
        }
    }

    higher_ids.sort();
    higher_ids.dedup();

    let renumber: Vec<RenumberEntry> = higher_ids
        .iter()
        .map(|&n| RenumberEntry {
            from: format!("TASK-T{:03}", n),
            to: format!("TASK-T{:03}", n - 1),
        })
        .collect();

    match args.confirm {
        None => {
            let token = generate_trm_token(id);
            let impact = RemoveImpact {
                id: id.to_string(),
                title: found_title,
                renumber,
                depends_on_updates: deps_affected,
                confirm_token: token.clone(),
                command: format!("dow task remove {} --confirm {}", id, token),
            };
            if human {
                println!("[dev-flow] Remove impact for {}", id);
                println!("  title: {}", impact.title);
                if !impact.renumber.is_empty() {
                    println!("  renumber:");
                    for r in &impact.renumber {
                        println!("    {} → {}", r.from, r.to);
                    }
                }
                if !impact.depends_on_updates.is_empty() {
                    println!("  depends_on updates: {:?}", impact.depends_on_updates);
                }
                println!("  confirm: {}", impact.command);
            } else {
                output::print_json(&impact);
            }
            Ok(0)
        }
        Some(ref token) => {
            if !token.starts_with("TRM-") || token.len() != 10 {
                return Err(DowError::new(
                    "invalid confirmation token format (expected TRM-xxxxxx)",
                    2,
                ));
            }
            let expected = generate_trm_token(id);
            if token != &expected {
                return Err(DowError::new("confirmation token mismatch", 1));
            }

            // 1. Remove the task entry from its file
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DowError::new(format!("cannot read file: {}", e), 1))?;
            let new_content = remove_task_entry(&content, id);

            // Update nums in frontmatter
            let new_count = count_tasks_in_content(&new_content);
            let new_content = update_frontmatter_nums(&new_content, new_count);

            if new_count == 0 {
                fs::remove_file(&file_path)
                    .map_err(|e| DowError::new(format!("cannot delete file: {}", e), 1))?;
            } else {
                fs::write(&file_path, &new_content)
                    .map_err(|e| DowError::new(format!("cannot write file: {}", e), 1))?;
            }

            // 2. Purge deleted ID from depends_on in all files, then renumber
            //    Use two-phase replacement to avoid cascade (T003→T002→T001)
            for path in &all_files_full {
                if let Ok(mut content) = fs::read_to_string(path) {
                    let mut changed = false;

                    // Remove references to the deleted task from depends_on
                    let removed_pattern = format!("\"{}\"", id);
                    if content.contains(&removed_pattern) {
                        content = purge_id_from_depends_on(&content, id);
                        changed = true;
                    }

                    // Phase 1: high→placeholder (reverse to avoid overwrite)
                    for &n in higher_ids.iter().rev() {
                        let old_id = format!("TASK-T{:03}", n);
                        let placeholder = format!("TASK-T__{}__", n);
                        if content.contains(&old_id) {
                            content = content.replace(&old_id, &placeholder);
                            changed = true;
                        }
                    }
                    // Phase 2: placeholder→final
                    for &n in &higher_ids {
                        let placeholder = format!("TASK-T__{}__", n);
                        let new_id = format!("TASK-T{:03}", n - 1);
                        content = content.replace(&placeholder, &new_id);
                    }

                    if changed {
                        fs::write(path, &content)
                            .map_err(|e| DowError::new(format!("cannot write file: {}", e), 1))?;
                    }
                }
            }

            Ok(0)
        }
    }
}

fn remove_task_entry(content: &str, target_id: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(id) = extract_task_id(line) {
            if id == target_id {
                // Skip entire entry (header + sub-fields)
                i += 1;
                while i < lines.len() {
                    let sub = lines[i].trim();
                    if sub.starts_with("- [ ]") || sub.starts_with("- [x]") || sub.is_empty() {
                        break;
                    }
                    i += 1;
                }
                // Skip trailing blank line
                if i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                continue;
            }
        }
        result.push(line.to_string());
        i += 1;
    }

    let mut out = result.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn purge_id_from_depends_on(content: &str, removed_id: &str) -> String {
    content
        .lines()
        .map(|line| {
            if !line.contains("depends_on:") || !line.contains(removed_id) {
                return line.to_string();
            }
            // Parse the inline list and remove the target ID
            if let Some(bracket_start) = line.find('[') {
                if let Some(bracket_end) = line.find(']') {
                    let prefix = &line[..bracket_start + 1];
                    let inner = &line[bracket_start + 1..bracket_end];
                    let items: Vec<&str> = inner
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| {
                            let unquoted = s.trim_matches('"');
                            unquoted != removed_id
                        })
                        .collect();
                    return format!("{}{}]", prefix, items.join(", "));
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_owning_task_id(content: &str, target_line: &str) -> Option<String> {
    let mut current_id: Option<String> = None;
    for line in content.lines() {
        if let Some(id) = extract_task_id(line) {
            current_id = Some(id);
        }
        if line == target_line {
            return current_id;
        }
    }
    None
}

fn generate_trm_token(id: &str) -> String {
    use sha2::{Digest, Sha256};
    let today = Local::now().format("%Y-%m-%d").to_string();
    let input = format!("TRM:{}:{}", id, today);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);
    format!("TRM-{}", &hex[..6])
}

// ─── List ────────────────────────────────────────────────────────────────────

fn list(args: TaskListArgs, human: bool) -> Result<i32, DowError> {
    let task_dir = resolve_task_dir()?;
    let all_files = all_task_files_including_done(&task_dir);

    let mut items: Vec<TaskListItem> = Vec::new();

    for path in &all_files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if let Ok(content) = fs::read_to_string(path) {
            let parsed = parse_all_tasks_in_file(&content, &filename);
            for task in parsed {
                if args.all || task.status == "pending" {
                    items.push(task);
                }
            }
        }
    }

    // Sort by ID
    items.sort_by(|a, b| a.id.cmp(&b.id));

    if human {
        if items.is_empty() {
            println!("[dev-flow] No tasks found");
        } else {
            println!("[dev-flow] Tasks: {}", items.len());
            println!("{}", "━".repeat(40));
            for item in &items {
                let marker = if item.status == "done" { "x" } else { " " };
                println!(
                    "[{}] {} - {} [{}] ({})",
                    marker, item.id, item.title, item.priority, item.r#type
                );
            }
        }
    } else {
        output::print_json(&items);
    }

    Ok(0)
}

fn parse_all_tasks_in_file(content: &str, filename: &str) -> Vec<TaskListItem> {
    let mut items = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let (is_done, id, title) = if trimmed.starts_with("- [ ] TASK-T") {
            if let Some(colon_pos) = trimmed[6..].find(':') {
                let id = trimmed[6..6 + colon_pos].to_string();
                let title = trimmed[6 + colon_pos + 1..].trim().to_string();
                (false, id, title)
            } else {
                continue;
            }
        } else if trimmed.starts_with("- [x] TASK-T") {
            if let Some(colon_pos) = trimmed[6..].find(':') {
                let id = trimmed[6..6 + colon_pos].to_string();
                let title = trimmed[6 + colon_pos + 1..].trim().to_string();
                (true, id, title)
            } else {
                continue;
            }
        } else {
            continue;
        };

        // Parse sub-fields
        let mut task_type = String::new();
        let mut priority = String::new();

        for j in (i + 1)..lines.len() {
            let sub = lines[j].trim();
            if sub.starts_with("- [ ]") || sub.starts_with("- [x]") {
                break;
            }
            if sub.starts_with("- type:") {
                task_type = sub.strip_prefix("- type:").unwrap().trim().to_string();
            } else if sub.starts_with("- priority:") {
                priority = sub.strip_prefix("- priority:").unwrap().trim().to_string();
            }
        }

        items.push(TaskListItem {
            id,
            title,
            r#type: task_type,
            priority,
            status: if is_done {
                "done".to_string()
            } else {
                "pending".to_string()
            },
            file: filename.to_string(),
        });
    }

    items
}

// ─── Show ────────────────────────────────────────────────────────────────────

fn show(id: &str, human: bool) -> Result<i32, DowError> {
    let task_dir = resolve_task_dir()?;
    let all_files = all_task_files_including_done(&task_dir);

    for path in &all_files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if let Ok(content) = fs::read_to_string(path) {
            if let Some(detail) = parse_task_detail(&content, id, &filename) {
                if human {
                    print_task_detail_human(&detail);
                } else {
                    output::print_json(&detail);
                }
                return Ok(0);
            }
        }
    }

    Err(DowError::new(format!("task {} not found", id), 1))
}

pub(crate) fn parse_task_detail(
    content: &str,
    target_id: &str,
    filename: &str,
) -> Option<TaskDetail> {
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let (is_done, id, title) = if trimmed.starts_with("- [ ] ") {
            let after = &trimmed[6..];
            if let Some(colon_pos) = after.find(':') {
                let id = after[..colon_pos].trim().to_string();
                let title = after[colon_pos + 1..].trim().to_string();
                (false, id, title)
            } else {
                continue;
            }
        } else if trimmed.starts_with("- [x] ") {
            let after = &trimmed[6..];
            if let Some(colon_pos) = after.find(':') {
                let id = after[..colon_pos].trim().to_string();
                let title = after[colon_pos + 1..].trim().to_string();
                (true, id, title)
            } else {
                continue;
            }
        } else {
            continue;
        };

        if id != target_id {
            continue;
        }

        // Parse all sub-fields for this task
        let mut task_type = String::new();
        let mut priority = String::new();
        let mut refs = String::new();
        let mut files_create = Vec::new();
        let mut files_modify = Vec::new();
        let mut files_test = Vec::new();
        let mut depends_on = Vec::new();
        let mut parallel = false;
        let mut complexity = String::new();
        let mut done_when = Vec::new();
        let mut in_files = false;
        let mut in_done_when = false;

        for j in (i + 1)..lines.len() {
            let sub = lines[j];
            let sub_trimmed = sub.trim();

            // Stop at next task item
            if sub_trimmed.starts_with("- [ ]") || sub_trimmed.starts_with("- [x]") {
                break;
            }

            if in_done_when {
                if sub_trimmed.starts_with("- ")
                    && !sub_trimmed.starts_with("- type:")
                    && !sub_trimmed.starts_with("- priority:")
                    && !sub_trimmed.starts_with("- refs:")
                    && !sub_trimmed.starts_with("- files:")
                    && !sub_trimmed.starts_with("- depends_on:")
                    && !sub_trimmed.starts_with("- parallel:")
                    && !sub_trimmed.starts_with("- complexity:")
                    && !sub_trimmed.starts_with("- done_when:")
                {
                    // This is a sub-item at done_when indentation level
                    if sub.starts_with("      - ") || sub.starts_with("      -") {
                        done_when.push(sub.trim().trim_start_matches("- ").to_string());
                        continue;
                    }
                }
                in_done_when = false;
            }

            if in_files {
                if sub_trimmed.starts_with("create:") {
                    files_create = parse_inline_list(sub_trimmed.strip_prefix("create:").unwrap());
                    continue;
                } else if sub_trimmed.starts_with("modify:") {
                    files_modify = parse_inline_list(sub_trimmed.strip_prefix("modify:").unwrap());
                    continue;
                } else if sub_trimmed.starts_with("test:") {
                    files_test = parse_inline_list(sub_trimmed.strip_prefix("test:").unwrap());
                    continue;
                } else if !sub_trimmed.is_empty() && !sub.starts_with("      ") {
                    in_files = false;
                }
            }

            if sub_trimmed.starts_with("- type:") {
                task_type = sub_trimmed
                    .strip_prefix("- type:")
                    .unwrap()
                    .trim()
                    .to_string();
            } else if sub_trimmed.starts_with("- priority:") {
                priority = sub_trimmed
                    .strip_prefix("- priority:")
                    .unwrap()
                    .trim()
                    .to_string();
            } else if sub_trimmed.starts_with("- refs:") {
                refs = sub_trimmed
                    .strip_prefix("- refs:")
                    .unwrap()
                    .trim()
                    .to_string();
            } else if sub_trimmed.starts_with("- files:") {
                in_files = true;
            } else if sub_trimmed.starts_with("- depends_on:") {
                depends_on = parse_inline_list(sub_trimmed.strip_prefix("- depends_on:").unwrap());
            } else if sub_trimmed.starts_with("- parallel:") {
                parallel = sub_trimmed.strip_prefix("- parallel:").unwrap().trim() == "true";
            } else if sub_trimmed.starts_with("- complexity:") {
                complexity = sub_trimmed
                    .strip_prefix("- complexity:")
                    .unwrap()
                    .trim()
                    .to_string();
            } else if sub_trimmed.starts_with("- done_when:") {
                in_done_when = true;
            }
        }

        return Some(TaskDetail {
            id,
            title,
            r#type: task_type,
            priority,
            status: if is_done {
                "done".to_string()
            } else {
                "pending".to_string()
            },
            refs,
            files: TaskFiles {
                create: files_create,
                modify: files_modify,
                test: files_test,
            },
            depends_on,
            parallel,
            complexity,
            done_when,
            file: filename.to_string(),
        });
    }

    None
}

pub(crate) fn parse_inline_list(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    // Handle [item1, item2] or ["item1", "item2"] format
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn print_task_detail_human(detail: &TaskDetail) {
    let marker = if detail.status == "done" { "x" } else { " " };
    println!("[{}] {} — {}", marker, detail.id, detail.title);
    println!("  type: {}", detail.r#type);
    println!("  priority: {}", detail.priority);
    if !detail.refs.is_empty() {
        println!("  refs: {}", detail.refs);
    }
    println!("  files:");
    println!("    create: {:?}", detail.files.create);
    println!("    modify: {:?}", detail.files.modify);
    println!("    test: {:?}", detail.files.test);
    if !detail.depends_on.is_empty() {
        println!("  depends_on: {:?}", detail.depends_on);
    }
    println!("  parallel: {}", detail.parallel);
    println!("  complexity: {}", detail.complexity);
    if !detail.done_when.is_empty() {
        println!("  done_when:");
        for crit in &detail.done_when {
            println!("    - {}", crit);
        }
    }
    println!("  file: {}", detail.file);
}

// ─── Done ────────────────────────────────────────────────────────────────────

fn done_multi(ids: &[String]) -> Result<i32, DowError> {
    if ids.is_empty() {
        return Err(DowError::new("dow task done requires at least one ID", 2));
    }
    for id in ids {
        done_single(id)?;
    }

    // Auto-revoke claims for completed tasks
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    for id in ids {
        let normalized = item_id::normalize_short(id);
        let _ = claim::revoke_claims(&doc_root_path, Some(&normalized));
    }

    Ok(0)
}

fn done_single(id: &str) -> Result<i32, DowError> {
    let task_dir = resolve_task_dir()?;
    let all_files = task_store::iter_task_files(&task_dir);

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [ ] {}:", id)) {
                test_runner::check_task_test(id)?;

                let new_content =
                    content.replace(&format!("- [ ] {}:", id), &format!("- [x] {}:", id));

                // Check if all tasks in file are done
                if !task_store::has_undone_items(&new_content) {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let done_filename = format!("done_{}", filename);
                    let mut done_path = task_dir.join(&done_filename);
                    // Avoid overwriting existing done_ file
                    if done_path.exists() {
                        let mut suffix = 2u32;
                        loop {
                            let alt =
                                format!("done_{}_{}.md", filename.trim_end_matches(".md"), suffix);
                            done_path = task_dir.join(&alt);
                            if !done_path.exists() {
                                break;
                            }
                            suffix += 1;
                        }
                    }

                    atomic_write(path, &new_content)?;
                    if let Err(error) = fs::rename(path, &done_path) {
                        let rollback = atomic_write(path, &content);
                        return match rollback {
                            Ok(()) => Err(DowError::new(
                                format!("cannot rename task file: {}", error),
                                1,
                            )),
                            Err(rollback_error) => Err(DowError::new(
                                format!(
                                    "cannot rename task file: {}; rollback failed: {}",
                                    error, rollback_error
                                ),
                                1,
                            )),
                        };
                    }
                } else {
                    atomic_write(path, &new_content)?;
                }

                return Ok(0);
            }
        }
    }

    Err(DowError::new(format!("pending task {} not found", id), 1))
}

fn atomic_write(path: &Path, content: &str) -> Result<(), DowError> {
    let parent = path
        .parent()
        .ok_or_else(|| DowError::new("cannot determine task file directory", 1))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("task");
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default(),
        filename
    );
    let temp_path = parent.join(format!(".dow-task-{}.tmp", suffix));

    if let Err(error) = fs::write(&temp_path, content) {
        return Err(DowError::new(
            format!("cannot write task file: {}", error),
            1,
        ));
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(DowError::new(
            format!("cannot write task file: {}", error),
            1,
        ));
    }
    Ok(())
}

// ─── Reopen ──────────────────────────────────────────────────────────────────

fn reopen(args: TaskReopenArgs, human: bool) -> Result<i32, DowError> {
    let task_dir = resolve_task_dir()?;
    let id = &args.id;

    // Find the task in done files or active files
    let all_files = all_task_files_including_done(&task_dir);
    let mut found_file: Option<PathBuf> = None;
    let mut found_title = String::new();

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [x] {}:", id)) {
                found_file = Some(path.clone());
                // Extract title
                for line in content.lines() {
                    if line.contains(&format!("- [x] {}:", id)) {
                        let after_id = line.split(':').skip(1).collect::<Vec<_>>().join(":");
                        found_title = after_id.trim().to_string();
                        break;
                    }
                }
                break;
            }
        }
    }

    let file_path = match found_file {
        Some(p) => p,
        None => return Err(DowError::new(format!("completed task {} not found", id), 1)),
    };

    let filename = file_path.file_name().unwrap().to_string_lossy().to_string();

    match args.confirm {
        None => {
            // Generate TRO token and output impact info
            let token = generate_tro_token(id);
            let impact = ReopenImpact {
                id: id.to_string(),
                title: found_title,
                file: filename,
                confirm_token: token.clone(),
                message: format!("To reopen, run: dow task reopen {} --confirm {}", id, token),
            };
            if human {
                println!("[dev-flow] Reopen impact for {}", id);
                println!("  title: {}", impact.title);
                println!("  file: {}", impact.file);
                println!(
                    "  confirm: dow task reopen {} --confirm {}",
                    id, impact.confirm_token
                );
            } else {
                output::print_json(&impact);
            }
            Ok(0)
        }
        Some(ref token) => {
            // Verify token format
            if !token.starts_with("TRO-") || token.len() != 10 {
                return Err(DowError::new(
                    "invalid confirmation token format (expected TRO-xxxxxx)",
                    2,
                ));
            }

            // Verify token matches
            let expected = generate_tro_token(id);
            if token != &expected {
                return Err(DowError::new("confirmation token mismatch", 1));
            }

            // Perform reopen: change [x] back to [ ]
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DowError::new(format!("cannot read file: {}", e), 1))?;
            let new_content = content.replace(&format!("- [x] {}:", id), &format!("- [ ] {}:", id));
            fs::write(&file_path, &new_content)
                .map_err(|e| DowError::new(format!("cannot write file: {}", e), 1))?;

            // If file has done_ prefix, remove it
            if filename.starts_with("done_") {
                let new_name = filename.strip_prefix("done_").unwrap();
                let new_path = task_dir.join(new_name);
                fs::rename(&file_path, &new_path)
                    .map_err(|e| DowError::new(format!("cannot rename file: {}", e), 1))?;
            }

            Ok(0)
        }
    }
}

fn generate_tro_token(id: &str) -> String {
    use sha2::{Digest, Sha256};
    let today = Local::now().format("%Y-%m-%d").to_string();
    let input = format!("TRO:{}:{}", id, today);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);
    format!("TRO-{}", &hex[..6])
}

// ─── Schema ──────────────────────────────────────────────────────────────────

fn schema(_human: bool) -> Result<i32, DowError> {
    let schema = serde_json::json!({
        "fields": {
            "id": {
                "type": "string",
                "pattern": "TASK-T[0-9]{3}",
                "description": "Auto-assigned sequential task ID"
            },
            "title": {
                "type": "string",
                "required": true,
                "description": "Task title (imperative form)"
            },
            "type": {
                "type": "string",
                "required": true,
                "enum": ["feat", "fix", "refactor", "docs", "perf", "test", "style"],
                "description": "Task type"
            },
            "priority": {
                "type": "string",
                "required": true,
                "enum": ["P0", "P1", "P2"],
                "description": "Priority level"
            },
            "refs": {
                "type": "string",
                "required": true,
                "description": "Reference to SPEC acceptance criteria or user request"
            },
            "files": {
                "type": "object",
                "required": true,
                "at_least_one": ["create", "modify"],
                "properties": {
                    "create": { "type": "array", "items": "string" },
                    "modify": { "type": "array", "items": "string" },
                    "test": { "type": "array", "items": "string" }
                }
            },
            "depends_on": {
                "type": "array",
                "required": true,
                "items": "string",
                "description": "Task IDs this task depends on"
            },
            "parallel": {
                "type": "boolean",
                "required": true,
                "description": "Whether this task can run in parallel with others"
            },
            "complexity": {
                "type": "string",
                "required": true,
                "enum": ["S", "M", "L"],
                "description": "Estimated complexity"
            },
            "done_when": {
                "type": "array",
                "required": true,
                "items": "string",
                "description": "Acceptance criteria for task completion"
            }
        },
        "file_format": {
            "name_pattern": "task_YYYY-MM-DD_N.md",
            "done_prefix": "done_task_YYYY-MM-DD_N.md",
            "frontmatter": ["title", "nums"],
            "item_format": "- [ ] TASK-TXXX: <title>"
        }
    });

    output::print_json(&schema);
    Ok(0)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn resolve_task_dir() -> Result<PathBuf, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    if !doc_root_path.join("STATUS.yaml").exists() {
        return Err(DowError::new("no .dev-doc found (run `dow init` first)", 1));
    }
    Ok(doc_root_path.join("task"))
}

/// Get all task details from a doc_root (used by claim dependency checking)
pub(crate) fn get_all_task_details(doc_root: &Path) -> Vec<TaskDetail> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return Vec::new();
    }
    let all_files = all_task_files_including_done(&task_dir);
    let mut results = Vec::new();

    for path in &all_files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if let Ok(content) = fs::read_to_string(path) {
            // Extract all task IDs from this file
            for line in content.lines() {
                let trimmed = line.trim();
                let id = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
                    rest.split(':').next().map(|s| s.trim().to_string())
                } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
                    rest.split(':').next().map(|s| s.trim().to_string())
                } else {
                    None
                };
                if let Some(id) = id {
                    if id.starts_with("TASK-T") {
                        if let Some(detail) = parse_task_detail(&content, &id, &filename) {
                            results.push(detail);
                        }
                    }
                }
            }
        }
    }

    results
}
