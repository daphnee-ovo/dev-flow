// dow/src/commands/
// ├── changelog_cmd.rs  -- dow changelog (list/add/schema)
//
// Related Docs:
// - [CHANGELOG spec](../../references/.dev-doc/CHANGELOG.md)

use crate::cli::{ChangelogAddArgs, ChangelogCommands};
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde_json::json;
use std::fs;

const REF_CHANGELOG: &str = include_str!("../../references/.dev-doc/CHANGELOG.md");

pub fn run(command: ChangelogCommands, human: bool) -> Result<i32, DowError> {
    match command {
        ChangelogCommands::List => list(human),
        ChangelogCommands::Add(args) => add(args),
        ChangelogCommands::Schema => schema(),
    }
}

fn list(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let path = doc_root_path.join("CHANGELOG.md");

    if !path.exists() {
        if human {
            println!("[dev-flow] CHANGELOG.md does not exist");
        } else {
            output::print_json(&json!([]));
        }
        return Ok(0);
    }

    let content = fs::read_to_string(&path).map_err(|e| DowError::new(e.to_string(), 1))?;

    let entries: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| line.starts_with("- "))
        .map(|line| {
            let text = line.strip_prefix("- ").unwrap_or(line).trim();
            // Try to split "YYYY-MM-DD rest..." into date + message
            if text.len() >= 10
                && text.chars().nth(4) == Some('-')
                && text.chars().nth(7) == Some('-')
            {
                let (date_part, rest) = text.split_at(10);
                let message = rest.trim();
                json!({ "date": date_part, "text": message })
            } else {
                json!({ "date": null, "text": text })
            }
        })
        .collect();

    if human {
        if entries.is_empty() {
            println!("[dev-flow] CHANGELOG is empty");
        } else {
            for entry in &entries {
                let date = entry
                    .get("date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("????-??-??");
                let text = entry.get("text").and_then(|v| v.as_str()).unwrap_or("");
                println!("  {} {}", date, text);
            }
        }
    } else {
        output::print_json(&entries);
    }

    Ok(0)
}

fn add(args: ChangelogAddArgs) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let path = doc_root_path.join("CHANGELOG.md");

    // Create CHANGELOG.md if it doesn't exist
    if !path.exists() {
        fs::write(&path, "# Changelog\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let content = fs::read_to_string(&path).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let new_entry = format!("- {} {}", today, args.text);

    // Append entry after the header (after the first non-empty line following "# Changelog")
    let mut lines: Vec<&str> = content.lines().collect();
    let insert_pos = find_insert_position(&lines);
    lines.insert(insert_pos, &new_entry);

    let result = lines.join("\n");
    // Ensure trailing newline
    let result = if result.ends_with('\n') {
        result
    } else {
        format!("{}\n", result)
    };

    fs::write(&path, result).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Silent on success
    Ok(0)
}

/// Find the position to insert a new changelog entry (after header, before existing entries)
fn find_insert_position(lines: &[&str]) -> usize {
    // Find the first line after the "# Changelog" header and any blank lines
    let mut past_header = false;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("# ") {
            past_header = true;
            continue;
        }
        if past_header {
            // Skip blank lines after header
            if line.trim().is_empty() {
                continue;
            }
            // Found the first content line; insert before it
            return i;
        }
    }
    // If no content found, append at end
    lines.len()
}

fn schema() -> Result<i32, DowError> {
    let parsed = parse_spec_to_json("changelog", REF_CHANGELOG);
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
    Ok(0)
}

// === JSON spec parsing ===

fn parse_spec_to_json(doc_type: &str, content: &str) -> serde_json::Value {
    let mut result = json!({ "type": doc_type });

    if let Some(title_line) = content.lines().find(|l| l.starts_with("# ")) {
        result["title"] = json!(title_line.trim_start_matches("# ").trim());
    }

    if let Some(path) = extract_section_first_line(content, "Path") {
        result["path"] = json!(path);
    }

    if let Some(template) = extract_code_block(content, "markdown") {
        result["template"] = json!(template);
    }

    if let Some(fields) = extract_table(content, "Field Description") {
        result["fields"] = fields;
    }

    let sections = extract_h2_sections(content);
    if !sections.is_empty() {
        result["sections"] = json!(sections);
    }

    let mut rules = Vec::new();
    for section_name in &[
        "Completion Rules",
        "Naming Rules",
        "Notes",
        "Additional Rules",
        "Cautions",
    ] {
        if let Some(items) = extract_bullet_list(content, section_name) {
            for item in items {
                rules.push(json!({ "category": section_name, "rule": item }));
            }
        }
    }
    if !rules.is_empty() {
        result["rules"] = json!(rules);
    }

    result["create_command"] = json!("dow changelog add --text \"...\"");
    result["create_hint"] =
        json!("Use 'dow changelog add' to append entries, 'dow changelog list' to read");

    result
}

// === Markdown parsing helpers ===

fn extract_section_first_line(content: &str, heading: &str) -> Option<String> {
    let marker = format!("## {}", heading);
    let mut in_section = false;
    for line in content.lines() {
        if line.starts_with(&marker) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_code_block(content: &str, lang: &str) -> Option<String> {
    let opener = format!("```{}", lang);
    let mut in_block = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if !in_block && line.trim().starts_with(&opener) {
            in_block = true;
            continue;
        }
        if in_block {
            if line.trim() == "```" {
                break;
            }
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn extract_table(content: &str, heading: &str) -> Option<serde_json::Value> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed == heading {
            start = Some(i + 1);
            break;
        }
    }

    let start = start?;
    let mut table_lines = Vec::new();
    let mut found_table = false;

    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.starts_with('|') {
            found_table = true;
            table_lines.push(trimmed);
        } else if found_table {
            break;
        } else if trimmed.starts_with('#') {
            break;
        }
    }

    if table_lines.len() < 3 {
        return None;
    }

    let headers: Vec<String> = table_lines[0]
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    let mut rows = Vec::new();
    for row_line in &table_lines[2..] {
        let cells: Vec<&str> = row_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect();

        let mut row = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = cells.get(i).unwrap_or(&"");
            row.insert(header.clone(), json!(val));
        }
        rows.push(serde_json::Value::Object(row));
    }

    Some(json!(rows))
}

fn extract_h2_sections(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| l.starts_with("## "))
        .map(|l| l.trim_start_matches("## ").trim().to_string())
        .collect()
}

fn extract_bullet_list(content: &str, heading: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed == heading {
            start = Some(i + 1);
            break;
        }
    }

    let start = start?;
    let mut items = Vec::new();

    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            break;
        }
        if let Some(item) = trimmed.strip_prefix("- ") {
            items.push(item.to_string());
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}
