// dow/src/commands/
// ├── prd.rs  -- dow prd (create/schema)
//
// Related Docs:
// - [PRD-FILE spec](../../references/binary/.dev-doc/PRD-FILE.md)

use crate::cli::PrdCommands;
use crate::core::{doc_root, yaml};
use crate::error::DowError;
use std::fs;

const REF_PRD: &str = include_str!("../../references/binary/.dev-doc/PRD-FILE.md");

pub fn run(command: PrdCommands, _human: bool) -> Result<i32, DowError> {
    match command {
        PrdCommands::Create => create(),
        PrdCommands::Schema => schema(),
    }
}

fn create() -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let path = doc_root_path.join("PRD.md");

    if path.exists() {
        return Err(DowError::new(
            format!(
                "{} already exists — run `dow iterate` to archive current iteration first",
                path.display()
            ),
            1,
        ));
    }

    let mode = get_current_mode();
    let content = prd_template(&mode);
    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Silent on success
    Ok(0)
}

fn schema() -> Result<i32, DowError> {
    let mode = get_current_mode();
    let mut parsed = parse_spec_to_json("prd", REF_PRD);
    filter_json_by_mode(&mut parsed, &mode);
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
    Ok(0)
}

// === Mode awareness ===

fn get_current_mode() -> String {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return "full".to_string();
    }
    let mode = yaml::get(&status_file, "mode")
        .ok()
        .flatten()
        .unwrap_or_default();
    if let Some(orig) = mode.strip_prefix("audit/") {
        orig.to_string()
    } else if mode.is_empty() {
        "full".to_string()
    } else {
        mode
    }
}

// === Template generation ===

fn prd_sections_for_mode(mode: &str) -> Vec<&'static str> {
    match mode {
        "fast" | "mvp" => vec![],
        "quick" => vec![
            "Background & Motivation",
            "Goals & Non-Goals",
            "Feature Requirements",
            "Success Metrics",
            "Open Questions",
        ],
        _ => vec![
            "Background & Motivation",
            "Goals & Non-Goals",
            "User Persona",
            "Feature Requirements",
            "User Flow",
            "Success Metrics",
            "Constraints & Assumptions",
            "Open Questions",
        ],
    }
}

fn prd_template(mode: &str) -> String {
    let sections = prd_sections_for_mode(mode);
    if sections.is_empty() {
        return format!(
            "# Product Requirements Document (PRD)\n\n> {} mode skips PRD phase.\n",
            mode
        );
    }

    let mut out = String::from("# Product Requirements Document (PRD)\n\n");
    for (i, sec) in sections.iter().enumerate() {
        out.push_str(&format!("## {}. {}\n", i + 1, sec));
        if *sec == "Feature Requirements" {
            out.push_str("### Must Have\n### Should Have\n### Could Have\n### Won't Have\n");
        }
        if *sec == "Goals & Non-Goals" {
            out.push_str("### Goals\n### Non-Goals (Explicitly out of scope)\n");
        }
        out.push('\n');
    }
    out
}

fn prd_template_with_hints(mode: &str) -> String {
    let sections = prd_sections_for_mode(mode);
    if sections.is_empty() {
        return format!(
            "# Product Requirements Document (PRD)\n\n> {} mode skips PRD phase.\n",
            mode
        );
    }

    let mut out = String::from("# Product Requirements Document (PRD)\n\n");
    for (i, sec) in sections.iter().enumerate() {
        out.push_str(&format!("## {}. {}\n", i + 1, sec));
        if *sec == "Feature Requirements" {
            out.push_str("### Must Have\n### Should Have\n### Could Have\n### Won't Have\n");
        }
        if *sec == "Goals & Non-Goals" {
            out.push_str("### Goals\n### Non-Goals (Explicitly out of scope)\n");
        }
        match *sec {
            "Background & Motivation" => {
                out.push_str("<Why are we doing this>\n");
            }
            "User Persona" => {
                out.push_str("<Who are the target users>\n");
            }
            "User Flow" => {
                out.push_str("<How users will use it>\n");
            }
            "Success Metrics" => {
                out.push_str("<How to measure success>\n");
            }
            "Constraints & Assumptions" => {
                out.push_str("<Prerequisites and limitations>\n");
            }
            "Open Questions" => {
                out.push_str("<Items to confirm>\n");
            }
            _ => {}
        }
        out.push('\n');
    }
    out
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

    if let Some(mode_table) = extract_table(content, "Required Sections by Mode") {
        result["mode_requirements"] = mode_table;
    }

    result["create_command"] = json!("dow prd create");
    result["create_hint"] =
        json!("Manual PRD.md creation is prohibited, must use this command to generate");

    result
}

fn filter_json_by_mode(parsed: &mut serde_json::Value, mode: &str) {
    use serde_json::json;

    if let Some(arr) = parsed.get("mode_requirements").and_then(|v| v.as_array()) {
        let filtered: Vec<serde_json::Value> = arr
            .iter()
            .filter(|row| {
                if let Some(val) = row.get(mode).and_then(|v| v.as_str()) {
                    val == "\u{2713}" || !val.contains('\u{2014}')
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        parsed["mode_requirements"] = json!(filtered);
    }

    let template = prd_template_with_hints(mode);
    parsed["template"] = json!(template.trim_end());
    parsed["current_mode"] = json!(mode);
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
