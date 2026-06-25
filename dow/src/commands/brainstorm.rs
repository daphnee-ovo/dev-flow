// dow/src/commands/
// ├── brainstorm.rs  -- dow brainstorm (create/schema)
//
// Related Docs:
// - [BRAINSTORM-FILE spec](../../references/.dev-doc/BRAINSTORM-FILE.md)

use crate::cli::BrainstormCommands;
use crate::core::doc_root;
use crate::error::DowError;
use std::fs;

const REF_BRAINSTORM: &str = include_str!("../../references/.dev-doc/BRAINSTORM-FILE.md");

pub fn run(command: BrainstormCommands, _human: bool) -> Result<i32, DowError> {
    match command {
        BrainstormCommands::Create => create(),
        BrainstormCommands::Schema => schema(),
    }
}

fn create() -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let path = doc_root_path.join("BRAINSTORM.md");

    if path.exists() {
        return Err(DowError::new(
            format!("{} already exists, will not overwrite", path.display()),
            1,
        ));
    }

    let content = brainstorm_template();
    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Silent on success
    Ok(0)
}

fn schema() -> Result<i32, DowError> {
    let parsed = parse_spec_to_json("brainstorm", REF_BRAINSTORM);
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
    Ok(0)
}

// === Template generation ===

fn brainstorm_template() -> String {
    r#"# Brainstorm Notes —

**Date**:

## Background & Purpose

## Key Decisions
| Decision Point | Choice | Rationale |
|----------------|--------|-----------|

## Design Approach

### Architecture

### Components

### Data Flow

### Error Handling

## Constraints & Boundaries

## Next Steps
"#
    .to_string()
}

// === JSON spec parsing ===

fn parse_spec_to_json(doc_type: &str, content: &str) -> serde_json::Value {
    use serde_json::json;

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

    result["create_command"] = json!("dow brainstorm create");
    result["create_hint"] =
        json!("Manual BRAINSTORM.md creation is prohibited, must use this command to generate");

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
    use serde_json::json;

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
