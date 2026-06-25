// dow/src/commands/
// ├── task.rs  -- dow task (task resource management)
//
// Related Docs:
// - [CLAUDE.md - Task File Format](../../../CLAUDE.md)

use crate::cli::{TaskCommands, TaskCreateArgs, TaskListArgs, TaskReopenArgs};
use crate::core::{doc_root, task_store};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct TaskInput {
    title: String,
    #[serde(default = "default_type")]
    r#type: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default)]
    refs: String,
    #[serde(default)]
    files_modify: Vec<String>,
    #[serde(default)]
    files_create: Vec<String>,
    #[serde(default)]
    files_test: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    parallel: bool,
    #[serde(default = "default_complexity")]
    complexity: String,
    #[serde(default)]
    done_when: Vec<String>,
}

fn default_type() -> String {
    "feat".to_string()
}
fn default_priority() -> String {
    "P1".to_string()
}
fn default_complexity() -> String {
    "S".to_string()
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
struct TaskDetail {
    id: String,
    title: String,
    r#type: String,
    priority: String,
    status: String,
    refs: String,
    files: TaskFiles,
    depends_on: Vec<String>,
    parallel: bool,
    complexity: String,
    done_when: Vec<String>,
    file: String,
}

#[derive(Serialize)]
struct TaskFiles {
    create: Vec<String>,
    modify: Vec<String>,
    test: Vec<String>,
}

#[derive(Serialize)]
struct ReopenImpact {
    id: String,
    title: String,
    file: String,
    confirm_token: String,
    message: String,
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

pub fn run(command: TaskCommands, human: bool) -> Result<i32, DowError> {
    match command {
        TaskCommands::Create(args) => create(args, human),
        TaskCommands::List(args) => list(args, human),
        TaskCommands::Show { id } => show(&id, human),
        TaskCommands::Done { ids } => done_multi(&ids),
        TaskCommands::Reopen(args) => reopen(args, human),
        TaskCommands::Schema => schema(human),
    }
}

// ─── Create ──────────────────────────────────────────────────────────────────

fn create(args: TaskCreateArgs, _human: bool) -> Result<i32, DowError> {
    let tasks = resolve_create_input(args)?;
    if tasks.is_empty() {
        return Err(DowError::new("no task input provided (use --title or pipe JSON to stdin)", 2));
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

    for (i, task) in tasks.iter().enumerate() {
        let id_num = next_id + i;
        let id_str = format!("TASK-T{:03}", id_num);
        let entry = format_task_entry(&id_str, task);
        content.push_str(&entry);
    }

    // Update frontmatter nums
    content = update_frontmatter_nums(&content, new_total);

    fs::write(&batch_file, &content)
        .map_err(|e| DowError::new(format!("cannot write task file: {}", e), 1))?;

    Ok(0)
}

fn resolve_create_input(args: TaskCreateArgs) -> Result<Vec<TaskInput>, DowError> {
    // Check stdin for JSON
    if let Some(stdin_data) = read_stdin_if_available() {
        let trimmed = stdin_data.trim();
        if trimmed.starts_with('[') {
            let tasks: Vec<TaskInput> = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON array from stdin: {}", e), 2))?;
            return Ok(tasks);
        } else if trimmed.starts_with('{') {
            let task: TaskInput = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON object from stdin: {}", e), 2))?;
            return Ok(vec![task]);
        }
    }

    // Use flags
    let title = match args.title {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };

    let task = TaskInput {
        title,
        r#type: args.task_type.unwrap_or_else(default_type),
        priority: args.priority.unwrap_or_else(default_priority),
        refs: args.refs.unwrap_or_default(),
        files_modify: split_comma(&args.files_modify),
        files_create: split_comma(&args.files_create),
        files_test: split_comma(&args.files_test),
        depends_on: split_comma(&args.depends_on),
        parallel: args.parallel,
        complexity: args.complexity,
        done_when: split_comma(&args.done_when),
    };

    Ok(vec![task])
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

fn split_comma(opt: &Option<String>) -> Vec<String> {
    match opt {
        Some(s) if !s.is_empty() => s.split(',').map(|x| x.trim().to_string()).collect(),
        _ => Vec::new(),
    }
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

fn all_task_files_including_done(task_dir: &Path) -> Vec<PathBuf> {
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
    let trimmed = line.trim();
    // Pattern: "- [ ] TASK-T###: ..." or "- [x] TASK-T###: ..."
    if trimmed.starts_with("- [") {
        let after_bracket = if trimmed.starts_with("- [ ] ") {
            &trimmed[6..]
        } else if trimmed.starts_with("- [x] ") {
            &trimmed[6..]
        } else {
            return None;
        };
        if let Some(colon_pos) = after_bracket.find(':') {
            let id = after_bracket[..colon_pos].trim().to_string();
            if id.starts_with("TASK-T") {
                return Some(id);
            }
        }
    }
    None
}

fn parse_task_num(id: &str) -> Option<usize> {
    id.strip_prefix("TASK-T")?.parse().ok()
}

fn find_or_create_batch_file(task_dir: &Path, today: &str) -> Result<PathBuf, DowError> {
    // Find existing batch file for today
    let mut max_seq = 0u32;
    let prefix = format!("task_{}_", today);

    if let Ok(entries) = fs::read_dir(task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) && name.ends_with(".md") {
                let seq_part = &name[prefix.len()..name.len() - 3];
                if let Ok(seq) = seq_part.parse::<u32>() {
                    if seq > max_seq {
                        max_seq = seq;
                    }
                }
            }
        }
    }

    if max_seq > 0 {
        // Append to existing file
        let path = task_dir.join(format!("task_{}_{}.md", today, max_seq));
        if path.exists() {
            return Ok(path);
        }
    }

    // Create new batch file
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
    lines.push(format!("      create: [{}]", format_string_list(&task.files_create)));
    lines.push(format!("      modify: [{}]", format_string_list(&task.files_modify)));
    lines.push(format!("      test: [{}]", format_string_list(&task.files_test)));
    lines.push(format!("  - depends_on: [{}]", format_string_list(&task.depends_on)));
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
                println!("[{}] {} - {} [{}] ({})", marker, item.id, item.title, item.priority, item.r#type);
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
            status: if is_done { "done".to_string() } else { "pending".to_string() },
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

fn parse_task_detail(content: &str, target_id: &str, filename: &str) -> Option<TaskDetail> {
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
                if sub_trimmed.starts_with("- ") && !sub_trimmed.starts_with("- type:")
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
                task_type = sub_trimmed.strip_prefix("- type:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- priority:") {
                priority = sub_trimmed.strip_prefix("- priority:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- refs:") {
                refs = sub_trimmed.strip_prefix("- refs:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- files:") {
                in_files = true;
            } else if sub_trimmed.starts_with("- depends_on:") {
                depends_on = parse_inline_list(sub_trimmed.strip_prefix("- depends_on:").unwrap());
            } else if sub_trimmed.starts_with("- parallel:") {
                parallel = sub_trimmed.strip_prefix("- parallel:").unwrap().trim() == "true";
            } else if sub_trimmed.starts_with("- complexity:") {
                complexity = sub_trimmed.strip_prefix("- complexity:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- done_when:") {
                in_done_when = true;
            }
        }

        return Some(TaskDetail {
            id,
            title,
            r#type: task_type,
            priority,
            status: if is_done { "done".to_string() } else { "pending".to_string() },
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

fn parse_inline_list(s: &str) -> Vec<String> {
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
    Ok(0)
}

fn done_single(id: &str) -> Result<i32, DowError> {
    let task_dir = resolve_task_dir()?;
    let all_files = task_store::iter_task_files(&task_dir);

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [ ] {}:", id)) {
                let new_content = content.replace(
                    &format!("- [ ] {}:", id),
                    &format!("- [x] {}:", id),
                );
                fs::write(path, &new_content)
                    .map_err(|e| DowError::new(format!("cannot write task file: {}", e), 1))?;

                // Check if all tasks in file are done
                if !task_store::has_undone_items(&new_content) {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let done_filename = format!("done_{}", filename);
                    let done_path = task_dir.join(&done_filename);
                    fs::rename(path, &done_path)
                        .map_err(|e| DowError::new(format!("cannot rename task file: {}", e), 1))?;
                }

                return Ok(0);
            }
        }
    }

    Err(DowError::new(format!("pending task {} not found", id), 1))
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
                println!("  confirm: dow task reopen {} --confirm {}", id, impact.confirm_token);
            } else {
                output::print_json(&impact);
            }
            Ok(0)
        }
        Some(ref token) => {
            // Verify token format
            if !token.starts_with("TRO-") || token.len() != 10 {
                return Err(DowError::new("invalid confirmation token format (expected TRO-xxxxxx)", 2));
            }

            // Verify token matches
            let expected = generate_tro_token(id);
            if token != &expected {
                return Err(DowError::new("confirmation token mismatch", 1));
            }

            // Perform reopen: change [x] back to [ ]
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DowError::new(format!("cannot read file: {}", e), 1))?;
            let new_content = content.replace(
                &format!("- [x] {}:", id),
                &format!("- [ ] {}:", id),
            );
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
                "enum": ["feat", "fix", "refactor", "docs", "perf", "test", "style"],
                "default": "feat",
                "description": "Task type"
            },
            "priority": {
                "type": "string",
                "enum": ["P0", "P1", "P2"],
                "default": "P1",
                "description": "Priority level"
            },
            "refs": {
                "type": "string",
                "description": "Reference to SPEC acceptance criteria or user request"
            },
            "files": {
                "type": "object",
                "properties": {
                    "create": { "type": "array", "items": "string" },
                    "modify": { "type": "array", "items": "string" },
                    "test": { "type": "array", "items": "string" }
                }
            },
            "depends_on": {
                "type": "array",
                "items": "string",
                "description": "Task IDs this task depends on"
            },
            "parallel": {
                "type": "boolean",
                "default": false,
                "description": "Whether this task can run in parallel with others"
            },
            "complexity": {
                "type": "string",
                "enum": ["S", "M", "L", "XL"],
                "default": "S",
                "description": "Estimated complexity"
            },
            "done_when": {
                "type": "array",
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
        return Err(DowError::new(
            "no .dev-doc found (run `dow init` first)",
            1,
        ));
    }
    Ok(doc_root_path.join("task"))
}
