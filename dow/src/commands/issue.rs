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

use crate::cli::{IssueCommands, IssueCreateArgs, IssueListArgs, IssueReopenArgs};
use crate::core::{doc_root, doc_validator};
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
struct IssueShowOutput {
    id: String,
    title: String,
    severity: String,
    location: String,
    description: String,
    reproduce: String,
    fix: String,
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

#[derive(Serialize)]
struct IssueSchemaOutput {
    fields: Vec<SchemaField>,
    file_format: String,
    id_format: String,
}

#[derive(Serialize)]
struct SchemaField {
    name: String,
    required: bool,
    r#type: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    valid_values: Vec<String>,
}

#[derive(Deserialize)]
struct IssueCreateInput {
    title: Option<String>,
    severity: Option<String>,
    location: Option<String>,
    desc: Option<String>,
    source: Option<String>,
    reproduce: Option<String>,
}

pub fn run(command: IssueCommands, human: bool) -> Result<i32, DowError> {
    match command {
        IssueCommands::Create(args) => create(args, human),
        IssueCommands::List(args) => list(args, human),
        IssueCommands::Show { id } => show(&id, human),
        IssueCommands::Close { id } => close(&id),
        IssueCommands::Reopen(args) => reopen(args, human),
        IssueCommands::Schema => schema(human),
    }
}

fn create(args: IssueCreateArgs, _human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let issue_dir = doc_root_path.join("issue");

    if !issue_dir.is_dir() {
        fs::create_dir_all(&issue_dir)
            .map_err(|e| DowError::new(format!("Failed to create issue directory: {}", e), 1))?;
    }

    // Detect stdin JSON or use flags
    let input = read_stdin_json_or_flags(&args)?;

    let title = input.title.ok_or_else(|| DowError::new("--title is required", 2))?;
    let severity = input.severity.unwrap_or_else(|| "P1".to_string());
    let location = input.location.unwrap_or_default();
    let desc = input.desc.unwrap_or_default();
    let source = input.source.unwrap_or_else(|| "other".to_string());
    let reproduce = input.reproduce.unwrap_or_default();

    // Validate severity
    let valid_severities = ["P0", "P1", "P2"];
    if !valid_severities.contains(&severity.as_str()) {
        return Err(DowError::new(
            format!("Invalid severity '{}', valid: P0/P1/P2", severity),
            2,
        ));
    }

    // Validate source
    let valid_sources = ["test", "devtest", "other", "audit"];
    if !valid_sources.contains(&source.as_str()) {
        return Err(DowError::new(
            format!("Invalid source '{}', valid: test/devtest/other/audit", source),
            2,
        ));
    }

    // Determine next issue ID (globally across all issue files)
    let next_id = next_issue_id(&issue_dir);

    // Determine next file sequence for this source+date
    let today = Local::now().format("%Y-%m-%d").to_string();
    let seq = next_file_seq(&issue_dir, &source, &today);

    let filename = format!("issue_{}_{}_{}.md", source, today, seq);
    let id_str = format!("ISSUE-I{:03}", next_id);

    let content = format!(
        "---\nsource: {}\nnums: 1\n---\n\n- [ ] {}：{}\n  - severity: {}\n  - location：{}\n  - description：{}\n  - reproduce：{}\n  - fix：\n",
        source, id_str, title, severity, location, desc, reproduce
    );

    fs::write(issue_dir.join(&filename), &content)
        .map_err(|e| DowError::new(format!("Failed to write issue file: {}", e), 1))?;

    // Silent on success
    Ok(0)
}

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
    let result = IssueListOutput { open: entries, total };

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
        println!("  File:        {}", result.file);
    } else {
        output::print_json(&result);
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
        return Err(DowError::new(
            format!("Issue {} is already closed", id),
            1,
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
    let total: usize = updated_content.lines().filter(|l| l.starts_with("- [")).count();
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
                    format!(
                        "Token mismatch: expected {}, got {}",
                        expected_token, token
                    ),
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
                fs::rename(&file_path, &new_path).map_err(|e| {
                    DowError::new(format!("Failed to rename issue file: {}", e), 1)
                })?;
            }

            // Silent on success
            Ok(0)
        }
    }
}

fn schema(_human: bool) -> Result<i32, DowError> {
    let result = IssueSchemaOutput {
        fields: vec![
            SchemaField {
                name: "title".to_string(),
                required: true,
                r#type: "string".to_string(),
                description: "Issue title".to_string(),
                valid_values: vec![],
            },
            SchemaField {
                name: "severity".to_string(),
                required: true,
                r#type: "enum".to_string(),
                description: "Issue severity level".to_string(),
                valid_values: vec!["P0".into(), "P1".into(), "P2".into()],
            },
            SchemaField {
                name: "location".to_string(),
                required: false,
                r#type: "string".to_string(),
                description: "Code location (file:line)".to_string(),
                valid_values: vec![],
            },
            SchemaField {
                name: "description".to_string(),
                required: false,
                r#type: "string".to_string(),
                description: "Issue description".to_string(),
                valid_values: vec![],
            },
            SchemaField {
                name: "reproduce".to_string(),
                required: false,
                r#type: "string".to_string(),
                description: "Steps to reproduce".to_string(),
                valid_values: vec![],
            },
            SchemaField {
                name: "fix".to_string(),
                required: false,
                r#type: "string".to_string(),
                description: "Fix description (filled on close)".to_string(),
                valid_values: vec![],
            },
            SchemaField {
                name: "source".to_string(),
                required: true,
                r#type: "enum".to_string(),
                description: "Issue source".to_string(),
                valid_values: vec![
                    "test".into(),
                    "devtest".into(),
                    "other".into(),
                    "audit".into(),
                ],
            },
        ],
        file_format: "issue_<source>_<YYYY-MM-DD>_<seq>.md".to_string(),
        id_format: "ISSUE-I### (3-digit zero-padded sequence)".to_string(),
    };

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

    // Normalize ID: accept both "ISSUE-I001" and "I001" shorthand
    let normalized_id = if id.starts_with("ISSUE-I") {
        id.to_string()
    } else if id.starts_with("I") && id[1..].chars().all(|c| c.is_ascii_digit()) {
        format!("ISSUE-{}", id)
    } else {
        format!("ISSUE-I{}", id)
    };

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
        let mut in_target = false;

        for line in content.lines() {
            if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line.contains(&normalized_id) {
                found_line = Some(line.to_string());
                in_target = true;
            } else if in_target
                && (line.starts_with("- [ ]") || line.starts_with("- [x]"))
            {
                // Next item, stop collecting fields
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
                } else if trimmed.starts_with("- location") {
                    // Handle both ：and :
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    location = val;
                } else if trimmed.starts_with("- description") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    description = val;
                } else if trimmed.starts_with("- reproduce") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    reproduce = val;
                } else if trimmed.starts_with("- fix") {
                    let val = trimmed
                        .splitn(2, |c| c == '：' || c == ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    fix = val;
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
                    if line.starts_with("- [") {
                        let title = line[5..].trim();
                        if let Some(num) = extract_issue_id_num(title) {
                            if num > max_id {
                                max_id = num;
                            }
                        }
                    }
                }
            }
        }
    }

    max_id + 1
}

/// Extract numeric ID from issue title (e.g. "ISSUE-I003：title" -> 3)
fn extract_issue_id_num(title: &str) -> Option<u32> {
    let prefix = "ISSUE-I";
    if !title.starts_with(prefix) {
        return None;
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
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

/// Read stdin if available (non-blocking check), parse as JSON; otherwise use CLI flags
fn read_stdin_json_or_flags(args: &IssueCreateArgs) -> Result<IssueCreateInput, DowError> {
    // Try reading stdin (non-terminal)
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
            let input: IssueCreateInput = serde_json::from_str(&buf).map_err(|e| {
                DowError::new(format!("Invalid stdin JSON: {}", e), 2)
            })?;
            return Ok(input);
        }
    }

    // Fallback to CLI flags
    Ok(IssueCreateInput {
        title: args.title.clone(),
        severity: args.severity.clone(),
        location: args.location.clone(),
        desc: args.desc.clone(),
        source: Some(args.source.clone()),
        reproduce: None,
    })
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
                let sev = line.split("severity:").nth(1).unwrap_or("").trim().to_string();
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
