// FrameworkTree
// issue.rs
// ├── struct IssueListOutput
// ├── struct IssueEntry
// ├── struct IssueItem
// ├── struct IssueFilesOutput
// ├── struct IssueShowOutput
// ├── struct IssueReopenPreview
// ├── struct IssueFilesInput
// ├── struct IssueCreateInput
// ├── enum IssueCreatePayload
// ├── struct IssueCreateRecord
// ├── struct IssueCreateGroup
// ├── run()
// ├── create()
// ├── validate_issue_create_input()
// ├── normalize_issue_files()
// ├── validate_issue_file_scope()
// ├── parse_issue_files_arg()
// ├── read_issue_stdin_if_available()
// ├── render_issue_entry()
// ├── create_issue_batch()
// ├── write_issue_batch()
// ├── struct IssueUpdateInput
// ├── update()
// ├── resolve_issue_update_input()
// ├── has_any_issue_update()
// ├── format_multiline_field()
// ├── replace_issue_entry_in_content()
// ├── struct IssueRemoveImpact
// ├── struct IssueRenumberEntry
// ├── remove()
// ├── remove_issue_entry()
// ├── update_issue_frontmatter_nums()
// ├── generate_irm_token()
// ├── list()
// ├── show()
// ├── close_multi()
// ├── close()
// ├── reopen()
// ├── schema()
// ├── struct ParsedIssueItem
// ├── find_issue_by_id()
// ├── next_issue_id()
// ├── next_file_seq()
// ├── generate_iro_token()
// ├── read_stdin_json_or_flags()
// └── parse_open_items()

// dow/src/commands/
// ├── issue.rs  -- dow issue (issue resource management)
//    ├── run()             -- dispatch subcommands
//    ├── create()          -- create issue from flags or stdin JSON
//    ├── list()            -- list open/all issues
//    ├── show()            -- show issue details by ID
//    ├── close()           -- close issue by ID
//    ├── reopen()          -- reopen closed issue (with token confirmation)
//    ├── schema()          -- output issue field JSON schema
//    ├── parse_open_items()
//    ├── find_issue_by_id()
//    ├── next_issue_id()
//    ├── next_file_seq()
//    ├── generate_iro_token()
//
// Related Docs:
// - [ISSUE Specification](../../../references/.dev-doc/ISSUE.md)

use crate::cli::{
    IssueCommands, IssueCreateArgs, IssueListArgs, IssueRemoveArgs, IssueReopenArgs,
    IssueUpdateArgs,
};
use crate::commands::task::{expand_file_list, parse_inline_list};
use crate::core::{claim, doc_root, doc_validator, item_id};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read as IoRead;
use std::path::PathBuf;

#[derive(Serialize)]
struct IssueListOutput {
    open: Vec<IssueEntry>,
    total: u32,
}

#[derive(Serialize)]
struct IssueEntry {
    file: String,
    items: Vec<IssueItem>,
}

#[derive(Serialize)]
struct IssueItem {
    title: String,
    severity: String,
}

#[derive(Serialize)]
struct IssueFilesOutput {
    create: Vec<String>,
    modify: Vec<String>,
}

#[derive(Serialize)]
struct IssueShowOutput {
    id: String,
    title: String,
    severity: String,
    location: String,
    description: String,
    reproduce: String,
    fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<IssueFilesOutput>,
    status: String,
    file: String,
}

#[derive(Serialize)]
struct IssueReopenPreview {
    id: String,
    impact: String,
    confirm_token: String,
    command: String,
}

#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct IssueFilesInput {
    create: Option<Vec<String>>,
    modify: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IssueCreateInput {
    title: Option<String>,
    severity: Option<String>,
    location: Option<String>,
    desc: Option<String>,
    source: Option<String>,
    reproduce: Option<String>,
    #[serde(default)]
    fix: Option<String>,
    files: IssueFilesInput,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum IssueCreatePayload {
    Many(Vec<IssueCreateInput>),
    One(IssueCreateInput),
}

pub(crate) struct IssueCreateRecord {
    pub(crate) title: String,
    pub(crate) severity: String,
    pub(crate) location: String,
    pub(crate) desc: String,
    pub(crate) source: String,
    pub(crate) reproduce: String,
    pub(crate) files_modify: Vec<String>,
    pub(crate) files_create: Vec<String>,
}

struct IssueCreateGroup {
    source: String,
    entries: Vec<(String, IssueCreateRecord)>,
}

pub fn run(command: IssueCommands, human: bool) -> Result<i32, DowError> {
    match command {
        IssueCommands::Create(args) => create(args, human),
        IssueCommands::Update(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            update(args)
        }
        IssueCommands::Remove(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            remove(args, human)
        }
        IssueCommands::List(args) => list(args, human),
        IssueCommands::Show { id } => show(&item_id::normalize_full(&id), human),
        IssueCommands::Close { ids } => {
            let ids: Vec<String> = ids.iter().map(|id| item_id::normalize_full(id)).collect();
            close_multi(&ids)
        }
        IssueCommands::Reopen(args) => {
            let mut args = args;
            args.id = item_id::normalize_full(&args.id);
            reopen(args, human)
        }
        IssueCommands::Schema => schema(human),
    }
}

fn create(args: IssueCreateArgs, _human: bool) -> Result<i32, DowError> {
    // Detect stdin JSON or use flags
    let inputs = read_stdin_json_or_flags(&args)?;
    if inputs.is_empty() {
        return Err(DowError::new("at least one issue is required", 2));
    }

    // Validate every item before creating directories or writing any files.
    let validated: Vec<IssueCreateRecord> = inputs
        .into_iter()
        .map(validate_issue_create_input)
        .collect::<Result<_, _>>()?;

    let ids = create_issue_batch(validated)?;

    for id in ids {
        println!("{}", id);
    }

    Ok(0)
}

fn validate_issue_create_input(input: IssueCreateInput) -> Result<IssueCreateRecord, DowError> {
    if input.fix.is_some() {
        return Err(DowError::new(
            "`fix` cannot be specified when creating an issue; record it later via `dow issue update <id> --fix \"...\"`",
            2,
        ));
    }

    let title = input
        .title
        .ok_or_else(|| DowError::new("--title is required", 2))?;
    let severity = input
        .severity
        .ok_or_else(|| DowError::new("--severity is required", 2))?;
    let location = input
        .location
        .ok_or_else(|| DowError::new("--location is required", 2))?;
    let desc = input
        .desc
        .ok_or_else(|| DowError::new("--desc is required", 2))?;
    let source = input
        .source
        .ok_or_else(|| DowError::new("--source is required", 2))?;
    let reproduce = input
        .reproduce
        .ok_or_else(|| DowError::new("--reproduce is required", 2))?;

    let valid_severities = ["P0", "P1", "P2"];
    if !valid_severities.contains(&severity.as_str()) {
        return Err(DowError::new(
            format!("Invalid severity '{}', valid: P0/P1/P2", severity),
            2,
        ));
    }

    let valid_sources = ["test", "other", "audit"];
    if !valid_sources.contains(&source.as_str()) {
        return Err(DowError::new(
            format!("Invalid source '{}', valid: test/other/audit", source),
            2,
        ));
    }

    let mut files = input.files;
    normalize_issue_files(&mut files);
    validate_issue_file_scope(&files, "issue create")?;

    Ok(IssueCreateRecord {
        title,
        severity,
        location,
        desc,
        source,
        reproduce,
        files_modify: files.modify.unwrap_or_default(),
        files_create: files.create.unwrap_or_default(),
    })
}

fn normalize_issue_files(files: &mut IssueFilesInput) {
    if let Some(values) = files.create.take() {
        files.create = Some(expand_file_list(values));
    }
    if let Some(values) = files.modify.take() {
        files.modify = Some(expand_file_list(values));
    }
}

fn validate_issue_file_scope(files: &IssueFilesInput, operation: &str) -> Result<(), DowError> {
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

fn parse_issue_files_arg(value: &str) -> Result<IssueFilesInput, DowError> {
    serde_json::from_str(value)
        .map_err(|e| DowError::new(format!("invalid --file JSON object: {}", e), 2))
}

fn read_issue_stdin_if_available() -> Option<String> {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        return None;
    }

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok()?;
    if buf.trim().is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn render_issue_entry(id: &str, issue: &IssueCreateRecord) -> String {
    let desc_formatted = format_multiline_field(&issue.desc, "  - description：");
    let reproduce_formatted = format_multiline_field(&issue.reproduce, "  - reproduce：");
    let mut files = String::new();
    if !issue.files_modify.is_empty() {
        files.push_str(&format!(
            "  - files_modify: [{}]\n",
            issue.files_modify.join(", ")
        ));
    }
    if !issue.files_create.is_empty() {
        files.push_str(&format!(
            "  - files_create: [{}]\n",
            issue.files_create.join(", ")
        ));
    }

    format!(
        "- [ ] {}：{}\n  - severity: {}\n  - location：{}\n{}\n{}\n  - fix：\n{}",
        id, issue.title, issue.severity, issue.location, desc_formatted, reproduce_formatted, files
    )
}

pub(crate) fn create_issue_batch(records: Vec<IssueCreateRecord>) -> Result<Vec<String>, DowError> {
    if records.is_empty() {
        return Err(DowError::new("at least one issue is required", 2));
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");
    if !issue_dir.is_dir() {
        fs::create_dir_all(&issue_dir)
            .map_err(|e| DowError::new(format!("Failed to create issue directory: {}", e), 1))?;
    }

    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut next_id = next_issue_id(&issue_dir);
    let mut groups: Vec<IssueCreateGroup> = Vec::new();
    let mut ids = Vec::new();

    for item in records {
        let id = format!("ISSUE-I{:03}", next_id);
        next_id += 1;
        ids.push(id.clone());

        if let Some(group) = groups.iter_mut().find(|group| group.source == item.source) {
            group.entries.push((id, item));
        } else {
            groups.push(IssueCreateGroup {
                source: item.source.clone(),
                entries: vec![(id, item)],
            });
        }
    }

    let mut files = Vec::new();
    for group in groups {
        let seq = next_file_seq(&issue_dir, &group.source, &today);
        let filename = format!("issue_{}_{}_{}.md", group.source, today, seq);
        let mut content = format!(
            "---\nsource: {}\nnums: {}\n---\n\n",
            group.source,
            group.entries.len()
        );
        for (id, item) in group.entries {
            content.push_str(&render_issue_entry(&id, &item));
        }
        files.push((filename, content));
    }

    write_issue_batch(&issue_dir, &files)?;
    Ok(ids)
}

fn write_issue_batch(issue_dir: &PathBuf, files: &[(String, String)]) -> Result<(), DowError> {
    let mut staged = Vec::new();

    for (filename, content) in files {
        let final_path = issue_dir.join(filename);
        if final_path.exists() {
            return Err(DowError::new(
                format!("issue file already exists: {}", final_path.display()),
                1,
            ));
        }

        let temp_path = issue_dir.join(format!(".{}.tmp", filename));
        if let Err(error) = fs::write(&temp_path, content) {
            for (_, path) in &staged {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_file(&temp_path);
            return Err(DowError::new(
                format!("Failed to stage issue file: {}", error),
                1,
            ));
        }
        staged.push((final_path, temp_path));
    }

    let mut committed = Vec::new();
    for (final_path, temp_path) in &staged {
        if let Err(error) = fs::rename(temp_path, final_path) {
            for path in &committed {
                let _ = fs::remove_file(path);
            }
            for (_, path) in &staged {
                let _ = fs::remove_file(path);
            }
            return Err(DowError::new(
                format!("Failed to commit issue file: {}", error),
                1,
            ));
        }
        committed.push(final_path.clone());
    }

    Ok(())
}

// ─── Update ─────────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct IssueUpdateInput {
    title: Option<String>,
    severity: Option<String>,
    location: Option<String>,
    desc: Option<String>,
    reproduce: Option<String>,
    fix: Option<String>,
    files: Option<IssueFilesInput>,
}

fn update(args: IssueUpdateArgs) -> Result<i32, DowError> {
    let id = args.id.clone();
    let mut input = resolve_issue_update_input(args)?;

    if !has_any_issue_update(&input) {
        return Err(DowError::new(
            "no fields to update (provide at least one --field)",
            2,
        ));
    }

    if let Some(files) = input.files.as_mut() {
        normalize_issue_files(files);
        validate_issue_file_scope(files, "issue update")?;
    }

    if let Some(ref s) = input.severity {
        let valid = ["P0", "P1", "P2"];
        if !valid.contains(&s.as_str()) {
            return Err(DowError::new(
                format!("invalid severity '{}', valid: P0/P1/P2", s),
                2,
            ));
        }
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let (file_path, _line_content, parsed) = find_issue_by_id(&issue_dir, &id)?;
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if filename.starts_with("closed_") {
        return Err(DowError::new(
            format!("cannot update closed issue {} (use 'reopen' first)", id),
            1,
        ));
    }

    // Merge fields (array fields use incremental logic)
    use crate::commands::task::apply_incremental;
    let new_title = input.title.clone().unwrap_or(parsed.title.clone());
    let new_severity = input.severity.clone().unwrap_or(parsed.severity.clone());
    let new_location = input.location.clone().unwrap_or(parsed.location.clone());
    let new_desc = input.desc.clone().unwrap_or(parsed.description.clone());
    let new_reproduce = input.reproduce.clone().unwrap_or(parsed.reproduce.clone());
    let new_fix = input.fix.clone().unwrap_or(parsed.fix.clone());
    let new_files_modify = match input.files.as_ref().and_then(|files| files.modify.clone()) {
        Some(values) => apply_incremental(values, parsed.files_modify.clone()),
        None => parsed.files_modify.clone(),
    };
    let new_files_create = match input.files.as_ref().and_then(|files| files.create.clone()) {
        Some(values) => apply_incremental(values, parsed.files_create.clone()),
        None => parsed.files_create.clone(),
    };
    if input.files.is_some() && new_files_modify.is_empty() && new_files_create.is_empty() {
        return Err(DowError::new(
            "issue update cannot remove the last files.create/files.modify path",
            2,
        ));
    }

    // Rebuild the entry
    let content = fs::read_to_string(&file_path)
        .map_err(|e| DowError::new(format!("cannot read issue file: {}", e), 1))?;

    let new_content = replace_issue_entry_in_content(
        &content,
        &parsed.id,
        &new_title,
        &new_severity,
        &new_location,
        &new_desc,
        &new_reproduce,
        &new_fix,
        &new_files_modify,
        &new_files_create,
    );

    fs::write(&file_path, &new_content)
        .map_err(|e| DowError::new(format!("cannot write issue file: {}", e), 1))?;

    Ok(0)
}

fn resolve_issue_update_input(args: IssueUpdateArgs) -> Result<IssueUpdateInput, DowError> {
    let has_flags = args.title.is_some()
        || args.severity.is_some()
        || args.location.is_some()
        || args.desc.is_some()
        || args.reproduce.is_some()
        || args.fix.is_some()
        || args.file.is_some();

    let stdin_data = read_issue_stdin_if_available();
    if has_flags && stdin_data.is_some() {
        return Err(DowError::new(
            "cannot combine issue update CLI options with stdin JSON; use one input source",
            2,
        ));
    }
    if let Some(stdin_data) = stdin_data {
        let trimmed = stdin_data.trim();
        if trimmed.starts_with('{') {
            let input: IssueUpdateInput = serde_json::from_str(trimmed)
                .map_err(|e| DowError::new(format!("invalid JSON from stdin: {}", e), 2))?;
            return Ok(input);
        }
        return Err(DowError::new(
            "issue update stdin input must be a JSON object",
            2,
        ));
    }

    Ok(IssueUpdateInput {
        title: args.title,
        severity: args.severity,
        location: args.location,
        desc: args.desc,
        reproduce: args.reproduce,
        fix: args.fix,
        files: args
            .file
            .map(|value| parse_issue_files_arg(&value))
            .transpose()?,
    })
}

fn has_any_issue_update(input: &IssueUpdateInput) -> bool {
    input.title.is_some()
        || input.severity.is_some()
        || input.location.is_some()
        || input.desc.is_some()
        || input.reproduce.is_some()
        || input.fix.is_some()
        || input.files.is_some()
}

/// 多行字段格式化：第一行紧跟 prefix，后续行用 4 空格缩进续行
fn format_multiline_field(value: &str, prefix: &str) -> String {
    let lines: Vec<&str> = value.lines().collect();
    if lines.len() <= 1 {
        return format!("{}{}", prefix, value);
    }
    let mut result = format!("{}{}", prefix, lines[0]);
    for line in &lines[1..] {
        result.push('\n');
        if line.is_empty() {
            result.push_str("    ");
        } else {
            result.push_str(&format!("    {}", line));
        }
    }
    result
}

fn replace_issue_entry_in_content(
    content: &str,
    target_id: &str,
    title: &str,
    severity: &str,
    location: &str,
    desc: &str,
    reproduce: &str,
    fix: &str,
    files_modify: &[String],
    files_create: &[String],
) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line.contains(target_id) {
            // Preserve checkbox state
            let checkbox = if line.starts_with("- [x]") {
                "- [x]"
            } else {
                "- [ ]"
            };
            result.push(format!("{} {}：{}", checkbox, target_id, title));
            result.push(format!("  - severity: {}", severity));
            result.push(format!("  - location：{}", location));
            result.push(format_multiline_field(desc, "  - description："));
            result.push(format_multiline_field(reproduce, "  - reproduce："));
            result.push(format_multiline_field(fix, "  - fix："));
            if !files_modify.is_empty() || !files_create.is_empty() {
                let modify_str = if files_modify.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{}]", files_modify.join(", "))
                };
                let create_str = if files_create.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{}]", files_create.join(", "))
                };
                result.push(format!("  - files_modify: {}", modify_str));
                result.push(format!("  - files_create: {}", create_str));
            }
            // Skip all old content until next issue entry or EOF
            i += 1;
            while i < lines.len() {
                if lines[i].starts_with("- [ ]") || lines[i].starts_with("- [x]") {
                    break;
                }
                i += 1;
            }
            continue;
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
struct IssueRemoveImpact {
    id: String,
    title: String,
    renumber: Vec<IssueRenumberEntry>,
    confirm_token: String,
    command: String,
}

#[derive(Serialize)]
struct IssueRenumberEntry {
    from: String,
    to: String,
}

fn remove(args: IssueRemoveArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let (file_path, _line_content, parsed) = find_issue_by_id(&issue_dir, &args.id)?;
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if filename.starts_with("closed_") {
        return Err(DowError::new(
            format!(
                "cannot remove closed issue {} (only open issues can be removed)",
                args.id
            ),
            1,
        ));
    }

    let target_num = item_id::parse(&parsed.id).map(|id| id.num()).unwrap_or(0);

    // Collect higher issue IDs for renumbering
    let mut higher_nums: Vec<u32> = Vec::new();
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") {
                    continue;
                }
                if !(name.starts_with("issue_") || name.starts_with("closed_issue_")) {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if let Some(parsed) = item_id::extract_from_line(line) {
                            if parsed.kind == item_id::ItemKind::Issue && parsed.num() > target_num
                            {
                                higher_nums.push(parsed.num());
                            }
                        }
                    }
                }
            }
        }
    }

    higher_nums.sort();
    higher_nums.dedup();

    let renumber: Vec<IssueRenumberEntry> = higher_nums
        .iter()
        .map(|&n| IssueRenumberEntry {
            from: format!("ISSUE-I{:03}", n),
            to: format!("ISSUE-I{:03}", n - 1),
        })
        .collect();

    match args.confirm {
        None => {
            let token = generate_irm_token(&args.id);
            let impact = IssueRemoveImpact {
                id: parsed.id.clone(),
                title: parsed.title.clone(),
                renumber,
                confirm_token: token.clone(),
                command: format!("dow issue remove {} --confirm {}", args.id, token),
            };
            if human {
                println!("[dev-flow] Remove impact for {}", parsed.id);
                println!("  title: {}", impact.title);
                if !impact.renumber.is_empty() {
                    println!("  renumber:");
                    for r in &impact.renumber {
                        println!("    {} → {}", r.from, r.to);
                    }
                }
                println!("  confirm: {}", impact.command);
            } else {
                output::print_json(&impact);
            }
            Ok(0)
        }
        Some(ref token) => {
            if !token.starts_with("IRM-") || token.len() != 10 {
                return Err(DowError::new(
                    "invalid confirmation token format (expected IRM-xxxxxx)",
                    2,
                ));
            }
            let expected = generate_irm_token(&args.id);
            if token != &expected {
                return Err(DowError::new("confirmation token mismatch", 1));
            }

            // 1. Remove the issue entry from its file
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DowError::new(format!("cannot read file: {}", e), 1))?;
            let new_content = remove_issue_entry(&content, &parsed.id);

            // Check if file is now empty of items
            let remaining = new_content.lines().filter(|l| l.starts_with("- [")).count();
            if remaining == 0 {
                fs::remove_file(&file_path)
                    .map_err(|e| DowError::new(format!("cannot delete file: {}", e), 1))?;
            } else {
                // Update nums in frontmatter
                let new_content = update_issue_frontmatter_nums(&new_content, remaining);
                fs::write(&file_path, &new_content)
                    .map_err(|e| DowError::new(format!("cannot write file: {}", e), 1))?;
            }

            // 2. Renumber higher issues across all files (two-phase to avoid cascade)
            if !higher_nums.is_empty() {
                if let Ok(entries) = fs::read_dir(&issue_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.ends_with(".md") {
                            continue;
                        }
                        if !(name.starts_with("issue_") || name.starts_with("closed_issue_")) {
                            continue;
                        }
                        if let Ok(mut content) = fs::read_to_string(entry.path()) {
                            let mut changed = false;
                            // Phase 1: old → placeholder
                            for &n in higher_nums.iter().rev() {
                                let old_id = format!("ISSUE-I{:03}", n);
                                let placeholder = format!("ISSUE-I__{}__", n);
                                if content.contains(&old_id) {
                                    content = content.replace(&old_id, &placeholder);
                                    changed = true;
                                }
                            }
                            // Phase 2: placeholder → final
                            for &n in &higher_nums {
                                let placeholder = format!("ISSUE-I__{}__", n);
                                let new_id = format!("ISSUE-I{:03}", n - 1);
                                content = content.replace(&placeholder, &new_id);
                            }
                            if changed {
                                fs::write(entry.path(), &content).map_err(|e| {
                                    DowError::new(format!("cannot write file: {}", e), 1)
                                })?;
                            }
                        }
                    }
                }
            }

            Ok(0)
        }
    }
}

fn remove_issue_entry(content: &str, target_id: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line.contains(target_id) {
            // Skip entry + sub-fields
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
        result.push(line.to_string());
        i += 1;
    }

    let mut out = result.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn update_issue_frontmatter_nums(content: &str, new_count: usize) -> String {
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

fn generate_irm_token(id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    "irm-salt".hash(&mut hasher);
    let hash = hasher.finish();
    format!("IRM-{:06x}", hash & 0xFFFFFF)
}

// ─── List ────────────────────────────────────────────────────────────────────

fn list(args: IssueListArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let validation_errors = doc_validator::validate_all_issues(&doc_root_path);
    if !validation_errors.is_empty() {
        let msg = doc_validator::format_errors_human(&validation_errors);
        return Err(DowError::new(msg, 1));
    }

    let mut entries = Vec::new();

    if issue_dir.is_dir() {
        if let Ok(files) = fs::read_dir(&issue_dir) {
            let mut file_list: Vec<_> = files
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if args.all {
                        (name.starts_with("issue_") || name.starts_with("closed_issue_"))
                            && name.ends_with(".md")
                    } else {
                        name.starts_with("issue_") && name.ends_with(".md")
                    }
                })
                .collect();
            file_list.sort_by_key(|e| e.file_name());

            for entry in file_list {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let items = parse_open_items(&content);
                    if !items.is_empty() || args.all {
                        entries.push(IssueEntry { file: name, items });
                    }
                }
            }
        }
    }

    let total = entries.iter().map(|e| e.items.len() as u32).sum();
    let result = IssueListOutput {
        open: entries,
        total,
    };

    if human {
        if result.total == 0 {
            println!("[dev-flow] No open issues");
        } else {
            println!("[dev-flow] Open issues: {}", result.total);
            println!("━━━━━━━━━━━━━━━━━━━━━━");
            for entry in &result.open {
                println!("{}:", entry.file);
                for item in &entry.items {
                    let sev = if item.severity.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", item.severity)
                    };
                    println!("  - {}{}", item.title, sev);
                }
            }
        }
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn show(id: &str, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let (file_path, _line_content, parsed) = find_issue_by_id(&issue_dir, id)?;
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let status = if filename.starts_with("closed_") {
        "closed"
    } else {
        "open"
    };

    let result = IssueShowOutput {
        id: parsed.id.clone(),
        title: parsed.title.clone(),
        severity: parsed.severity.clone(),
        location: parsed.location.clone(),
        description: parsed.description.clone(),
        reproduce: parsed.reproduce.clone(),
        fix: parsed.fix.clone(),
        files: if parsed.files_modify.is_empty() && parsed.files_create.is_empty() {
            None
        } else {
            Some(IssueFilesOutput {
                create: parsed.files_create.clone(),
                modify: parsed.files_modify.clone(),
            })
        },
        status: status.to_string(),
        file: filename,
    };

    if human {
        println!("[dev-flow] Issue: {}", result.id);
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Title:       {}", result.title);
        println!("  Severity:    {}", result.severity);
        println!("  Status:      {}", result.status);
        println!("  Location:    {}", result.location);
        println!("  Description: {}", result.description);
        println!("  Reproduce:   {}", result.reproduce);
        println!("  Fix:         {}", result.fix);
        if let Some(files) = &result.files {
            if !files.modify.is_empty() {
                println!("  Files modify: {}", files.modify.join(", "));
            }
            if !files.create.is_empty() {
                println!("  Files create: {}", files.create.join(", "));
            }
        }
        println!("  File:        {}", result.file);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn close_multi(ids: &[String]) -> Result<i32, DowError> {
    if ids.is_empty() {
        return Err(DowError::new("dow issue close requires at least one ID", 2));
    }
    for id in ids {
        close(id)?;
    }

    // Auto-revoke claims for closed issues
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    for id in ids {
        let normalized = item_id::normalize_short(id);
        let _ = claim::revoke_claims(&doc_root_path, Some(&normalized));
    }

    Ok(0)
}

fn close(id: &str) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let (file_path, _line_content, parsed) = find_issue_by_id(&issue_dir, id)?;
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Must be open (not already closed)
    if filename.starts_with("closed_") {
        return Err(DowError::new(format!("Issue {} is already closed", id), 1));
    }

    // fix field must be filled before closing
    if parsed.fix.trim().is_empty() {
        return Err(DowError::new(
            format!("cannot close {}: fix field is empty (use `dow issue update {} --fix \"...\"` to describe the fix first)", id, id),
            2,
        ));
    }

    // Read file, change "- [ ] ISSUE-I###" to "- [x] ISSUE-I###" for matching ID
    let content = fs::read_to_string(&file_path)
        .map_err(|e| DowError::new(format!("Failed to read issue file: {}", e), 1))?;

    let id_prefix = format!("{}：", parsed.id);
    let id_prefix_alt = format!("{}: ", parsed.id);
    let new_content = content
        .lines()
        .map(|line| {
            if line.starts_with("- [ ]")
                && (line.contains(&id_prefix) || line.contains(&id_prefix_alt))
            {
                line.replacen("- [ ]", "- [x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Write back
    let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };
    fs::write(&file_path, &final_content)
        .map_err(|e| DowError::new(format!("Failed to write issue file: {}", e), 1))?;

    // Check if all issues in file are now closed; if so rename to closed_
    let updated_content = fs::read_to_string(&file_path).unwrap_or_default();
    let total: usize = updated_content
        .lines()
        .filter(|l| l.starts_with("- ["))
        .count();
    let done: usize = updated_content
        .lines()
        .filter(|l| l.starts_with("- [x]"))
        .count();

    if total > 0 && total == done {
        let new_filename = format!("closed_{}", filename);
        let new_path = issue_dir.join(&new_filename);
        fs::rename(&file_path, &new_path)
            .map_err(|e| DowError::new(format!("Failed to rename issue file: {}", e), 1))?;
    }

    // Silent on success
    Ok(0)
}

fn reopen(args: IssueReopenArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    let (file_path, _line_content, parsed) = find_issue_by_id(&issue_dir, &args.id)?;
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Must be closed
    let is_checked = _line_content.starts_with("- [x]");
    if !is_checked {
        return Err(DowError::new(
            format!("Issue {} is not closed, cannot reopen", args.id),
            1,
        ));
    }

    match args.confirm {
        None => {
            // Preview mode: output impact + generate token
            let token = generate_iro_token(&args.id);
            let result = IssueReopenPreview {
                id: parsed.id.clone(),
                impact: format!(
                    "Reopening {} will change its status from closed to open",
                    parsed.id
                ),
                confirm_token: token.clone(),
                command: format!("dow issue reopen {} --confirm {}", args.id, token),
            };

            if human {
                println!("[dev-flow] Reopen preview: {}", result.id);
                println!("━━━━━━━━━━━━━━━━━━━━━━");
                println!("  Impact: {}", result.impact);
                println!("  Confirm with: {}", result.command);
            } else {
                output::print_json(&result);
            }

            Ok(0)
        }
        Some(ref token) => {
            // Verify token format
            if !token.starts_with("IRO-") || token.len() != 10 {
                return Err(DowError::new(
                    format!("Invalid confirmation token: {}", token),
                    2,
                ));
            }

            // Verify token matches expected
            let expected_token = generate_iro_token(&args.id);
            if *token != expected_token {
                return Err(DowError::new(
                    format!("Token mismatch: expected {}, got {}", expected_token, token),
                    1,
                ));
            }

            // Change "- [x]" to "- [ ]" for matching ID
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DowError::new(format!("Failed to read issue file: {}", e), 1))?;

            let id_prefix = format!("{}：", parsed.id);
            let id_prefix_alt = format!("{}: ", parsed.id);
            let new_content = content
                .lines()
                .map(|line| {
                    if line.starts_with("- [x]")
                        && (line.contains(&id_prefix) || line.contains(&id_prefix_alt))
                    {
                        line.replacen("- [x]", "- [ ]", 1)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
                format!("{}\n", new_content)
            } else {
                new_content
            };
            fs::write(&file_path, &final_content)
                .map_err(|e| DowError::new(format!("Failed to write issue file: {}", e), 1))?;

            // Remove closed_ prefix if present
            if filename.starts_with("closed_") {
                let new_filename = filename.strip_prefix("closed_").unwrap();
                let new_path = issue_dir.join(new_filename);
                fs::rename(&file_path, &new_path)
                    .map_err(|e| DowError::new(format!("Failed to rename issue file: {}", e), 1))?;
            }

            // Silent on success
            Ok(0)
        }
    }
}

fn schema(_human: bool) -> Result<i32, DowError> {
    let result = serde_json::json!({
        "fields": [
            {
                "name": "title",
                "required": true,
                "type": "string",
                "description": "Issue title"
            },
            {
                "name": "severity",
                "required": true,
                "type": "enum",
                "description": "Issue severity level",
                "valid_values": ["P0", "P1", "P2"]
            },
            {
                "name": "location",
                "required": true,
                "type": "string",
                "description": "Code location (file:line)"
            },
            {
                "name": "description",
                "required": true,
                "type": "string",
                "description": "Issue description"
            },
            {
                "name": "reproduce",
                "required": true,
                "type": "string",
                "description": "Steps to reproduce"
            },
            {
                "name": "fix",
                "required": false,
                "type": "string",
                "description": "Fix description (filled on close)"
            },
            {
                "name": "source",
                "required": true,
                "type": "enum",
                "description": "Issue source",
                "valid_values": ["test", "other", "audit"]
            },
            {
                "name": "files",
                "required": true,
                "type": "object",
                "description": "File scope; at least one non-empty create or modify list is required",
                "at_least_one": ["create", "modify"],
                "properties": {
                    "create": {"type": "array", "items": "string"},
                    "modify": {"type": "array", "items": "string"}
                }
            }
        ],
        "file_format": "issue_<source>_<YYYY-MM-DD>_<seq>.md",
        "id_format": "ISSUE-I### (3-digit zero-padded sequence)"
    });

    output::print_json(&result);
    Ok(0)
}

// ==================== Helpers ====================

/// Parsed issue item fields
struct ParsedIssueItem {
    id: String,
    title: String,
    severity: String,
    location: String,
    description: String,
    reproduce: String,
    fix: String,
    files_modify: Vec<String>,
    files_create: Vec<String>,
}

/// Find issue by ID across all issue files. Returns (file_path, matching_line, parsed_fields).
fn find_issue_by_id(
    issue_dir: &PathBuf,
    id: &str,
) -> Result<(PathBuf, String, ParsedIssueItem), DowError> {
    if !issue_dir.is_dir() {
        return Err(DowError::new("Issue directory does not exist", 1));
    }

    let entries = fs::read_dir(issue_dir)
        .map_err(|e| DowError::new(format!("Failed to read issue directory: {}", e), 1))?;

    let normalized_id = item_id::normalize_full(id);

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        if !(name.starts_with("issue_") || name.starts_with("closed_issue_")) {
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Search for the matching ID line
        let mut found_line: Option<String> = None;
        let mut severity = String::new();
        let mut location = String::new();
        let mut description = String::new();
        let mut reproduce = String::new();
        let mut fix = String::new();
        let mut files_modify: Vec<String> = Vec::new();
        let mut files_create: Vec<String> = Vec::new();
        let mut in_target = false;
        let mut last_field = "";

        for line in content.lines() {
            if (line.starts_with("- [ ]") || line.starts_with("- [x]"))
                && line.contains(&normalized_id)
            {
                found_line = Some(line.to_string());
                in_target = true;
                last_field = "";
            } else if in_target && (line.starts_with("- [ ]") || line.starts_with("- [x]")) {
                break;
            } else if in_target {
                let trimmed = line.trim();
                if trimmed.starts_with("- severity:") {
                    severity = trimmed
                        .split("severity:")
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    last_field = "severity";
                } else if trimmed.starts_with("- location") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    location = val;
                    last_field = "location";
                } else if trimmed.starts_with("- description") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    description = val;
                    last_field = "description";
                } else if trimmed.starts_with("- reproduce") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    reproduce = val;
                    last_field = "reproduce";
                } else if trimmed.starts_with("- fix") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    fix = val;
                    last_field = "fix";
                } else if trimmed.starts_with("- files_modify:") {
                    files_modify = parse_inline_list(trimmed.splitn(2, ':').nth(1).unwrap_or(""));
                    last_field = "";
                } else if trimmed.starts_with("- files_create:") {
                    files_create = parse_inline_list(trimmed.splitn(2, ':').nth(1).unwrap_or(""));
                    last_field = "";
                } else if !last_field.is_empty() {
                    // 续行：非已知字段头的行属于上一个字段
                    let continuation = if line.starts_with("    ") {
                        &line[4..]
                    } else {
                        trimmed
                    };
                    match last_field {
                        "description" => {
                            description.push('\n');
                            description.push_str(continuation);
                        }
                        "reproduce" => {
                            reproduce.push('\n');
                            reproduce.push_str(continuation);
                        }
                        "fix" => {
                            fix.push('\n');
                            fix.push_str(continuation);
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(ref line) = found_line {
            // Extract title from the line: "- [ ] ISSUE-I001：title" or "- [x] ISSUE-I001：title"
            let after_checkbox = &line[5..].trim().to_string();
            let title = after_checkbox
                .splitn(2, |c| c == '：' || c == ':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();

            return Ok((
                entry.path(),
                line.clone(),
                ParsedIssueItem {
                    id: normalized_id,
                    title,
                    severity,
                    location,
                    description,
                    reproduce,
                    fix,
                    files_modify,
                    files_create,
                },
            ));
        }
    }

    Err(DowError::new(format!("Issue {} not found", id), 1))
}

/// Get next available issue ID number across all issue files
fn next_issue_id(issue_dir: &PathBuf) -> u32 {
    let mut max_id: u32 = 0;

    if !issue_dir.is_dir() {
        return 1;
    }

    if let Ok(entries) = fs::read_dir(issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if !(name.starts_with("issue_") || name.starts_with("closed_issue_")) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for line in content.lines() {
                    if let Some(parsed) = item_id::extract_from_line(line) {
                        if parsed.kind == item_id::ItemKind::Issue && parsed.num() > max_id {
                            max_id = parsed.num();
                        }
                    }
                }
            }
        }
    }

    max_id + 1
}

/// Get next file sequence number for a given source + date combination
fn next_file_seq(issue_dir: &PathBuf, source: &str, date: &str) -> u32 {
    let mut max_seq: u32 = 0;
    let prefix = format!("issue_{}_{}_", source, date);
    let closed_prefix = format!("closed_issue_{}_{}_", source, date);

    if let Ok(entries) = fs::read_dir(issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let seq_str = if let Some(rest) = name.strip_prefix(&prefix) {
                rest.strip_suffix(".md")
            } else if let Some(rest) = name.strip_prefix(&closed_prefix) {
                rest.strip_suffix(".md")
            } else {
                None
            };

            if let Some(s) = seq_str {
                if let Ok(n) = s.parse::<u32>() {
                    if n > max_seq {
                        max_seq = n;
                    }
                }
            }
        }
    }

    max_seq + 1
}

/// Generate deterministic IRO token from issue ID
fn generate_iro_token(id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    // Mix with a salt for uniqueness
    "iro-salt".hash(&mut hasher);
    let hash = hasher.finish();
    format!("IRO-{:06x}", hash & 0xFFFFFF)
}

/// Read stdin if available (non-blocking check), parse as JSON; otherwise use CLI flags.
fn read_stdin_json_or_flags(args: &IssueCreateArgs) -> Result<Vec<IssueCreateInput>, DowError> {
    let has_flags = args.title.is_some()
        || args.severity.is_some()
        || args.location.is_some()
        || args.desc.is_some()
        || args.source.is_some()
        || args.reproduce.is_some()
        || args.file.is_some();

    let stdin_data = read_issue_stdin_if_available();
    if has_flags && stdin_data.is_some() {
        return Err(DowError::new(
            "cannot combine issue CLI options with stdin JSON; use one input source",
            2,
        ));
    }
    if let Some(stdin_data) = stdin_data {
        let payload: IssueCreatePayload = serde_json::from_str(stdin_data.trim())
            .map_err(|e| DowError::new(format!("Invalid stdin JSON: {}", e), 2))?;
        return Ok(match payload {
            IssueCreatePayload::One(input) => vec![input],
            IssueCreatePayload::Many(inputs) => inputs,
        });
    }

    // Fallback to CLI flags
    let file = args
        .file
        .as_deref()
        .ok_or_else(|| DowError::new("--file is required", 2))
        .and_then(parse_issue_files_arg)?;
    Ok(vec![IssueCreateInput {
        title: args.title.clone(),
        severity: args.severity.clone(),
        location: args.location.clone(),
        desc: args.desc.clone(),
        source: args.source.clone(),
        reproduce: args.reproduce.clone(),
        fix: None,
        files: file,
    }])
}

fn parse_open_items(content: &str) -> Vec<IssueItem> {
    let mut items = Vec::new();
    let mut current_title = String::new();
    let mut in_open = false;

    for line in content.lines() {
        if line.starts_with("- [ ]") {
            in_open = true;
            current_title = line[5..].trim().to_string();
        } else if line.starts_with("- [x]") {
            in_open = false;
            current_title.clear();
        } else if in_open {
            if line.contains("severity:") {
                let sev = line
                    .split("severity:")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                items.push(IssueItem {
                    title: current_title.clone(),
                    severity: sev,
                });
                in_open = false;
                current_title.clear();
            }
        }
    }

    if in_open && !current_title.is_empty() {
        items.push(IssueItem {
            title: current_title,
            severity: String::new(),
        });
    }

    items
}
