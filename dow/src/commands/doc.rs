// dow/src/commands/
// ├── doc.rs  -- dow doc (Document template generation + documentation specification query)

use crate::cli::DocArgs;
use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

// Embedded specification files from references/.dev-doc/
const REF_TASK: &str = include_str!("../../references/.dev-doc/TASK-FILE.md");
const REF_ISSUE: &str = include_str!("../../references/.dev-doc/ISSUE.md");
const REF_PRD: &str = include_str!("../../references/.dev-doc/PRD-FILE.md");
const REF_SPEC: &str = include_str!("../../references/.dev-doc/SPEC-FILE.md");
const REF_TEST: &str = include_str!("../../references/.dev-doc/TEST.md");
const REF_BRAINSTORM: &str = include_str!("../../references/.dev-doc/BRAINSTORM-FILE.md");
const REF_CHANGELOG: &str = include_str!("../../references/.dev-doc/CHANGELOG.md");

#[derive(Serialize)]
struct DocOutput {
    created: String,
    #[serde(rename = "type")]
    doc_type: String,
    slots: u32,
}

/// Valid document types
const VALID_TYPES: &[&str] = &["task", "issue", "prd", "spec", "test", "brainstorm", "changelog", "init", "check-sync", "list"];

/// Valid issue sources
const VALID_SOURCES: &[&str] = &["test", "devtest", "other", "audit"];

pub fn run(args: DocArgs, human: bool) -> Result<i32, DowError> {
    let doc_type = args.doc_type.to_lowercase();

    if !VALID_TYPES.contains(&doc_type.as_str()) {
        return Err(DowError::new(
            format!(
                "Unknown document type: {} (available: {})",
                doc_type,
                VALID_TYPES.join("/")
            ),
            1,
        ));
    }

    // doc init: Generate persistent document skeleton
    if doc_type == "init" {
        return run_doc_init(args.project_name.as_deref(), human);
    }

    // doc check-sync: Check document sync status
    if doc_type == "check-sync" {
        return run_doc_check_sync(args.since.as_deref(), human);
    }

    // doc list: List registered documents
    if doc_type == "list" {
        return run_doc_list(human);
    }

    // --md / --json: Output document specification (spec/prd filtered by current mode)
    if args.md || args.json {
        let mode = get_current_mode();
        return output_spec(&doc_type, args.md, &mode);
    }

    // Validate --source (only used for issue type)
    if let Some(ref src) = args.source {
        if !VALID_SOURCES.contains(&src.as_str()) {
            return Err(DowError::new(
                format!(
                    "Invalid issue source: {} (available: {})",
                    src,
                    VALID_SOURCES.join("/")
                ),
                1,
            ));
        }
    }

    // Default: Create template file
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    let mode = get_current_mode();
    let (path, slots) = match doc_type.as_str() {
        "task" => create_task(&doc_root_path, args.count)?,
        "issue" => create_issue(&doc_root_path, args.count, args.source.as_deref())?,
        "prd" => create_single(&doc_root_path, "PRD.md", prd_template(&mode))?,
        "spec" => create_single(&doc_root_path, "SPEC.md", spec_template(&mode))?,
        "test" => create_single(&doc_root_path, "TEST.md", test_template())?,
        "brainstorm" => create_single(&doc_root_path, "BRAINSTORM.md", brainstorm_template())?,
        "changelog" => create_single(&doc_root_path, "CHANGELOG.md", changelog_template())?,
        _ => unreachable!(),
    };

    let result = DocOutput {
        created: path,
        doc_type,
        slots,
    };

    if human {
        println!("[dev-flow] Document created: {}", result.created);
        match result.doc_type.as_str() {
            "task" | "issue" => {
                println!(
                    "  Hint: Use -n <count> to specify number of entries in template, e.g. dow doc {} -n 5",
                    result.doc_type
                );
            }
            _ => {}
        }
        println!(
            "  Hint: Use dow doc {} --md or dow doc {} --json to view document format specification",
            result.doc_type, result.doc_type
        );
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

/// Output document specification (--md or --json), spec/prd filtered by mode
fn output_spec(doc_type: &str, as_md: bool, mode: &str) -> Result<i32, DowError> {
    let content = get_reference(doc_type);

    if as_md {
        let filtered = filter_md_by_mode(doc_type, content, mode);
        println!("{}", filtered);
    } else {
        let mut parsed = parse_spec_to_json(doc_type, content);
        filter_json_by_mode(&mut parsed, mode);
        println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
    }

    Ok(0)
}

/// Get reference document content for the corresponding type
fn get_reference(doc_type: &str) -> &'static str {
    match doc_type {
        "task" => REF_TASK,
        "issue" => REF_ISSUE,
        "prd" => REF_PRD,
        "spec" => REF_SPEC,
        "test" => REF_TEST,
        "brainstorm" => REF_BRAINSTORM,
        "changelog" => REF_CHANGELOG,
        _ => unreachable!(),
    }
}

/// Parse markdown specification into structured JSON
fn parse_spec_to_json(doc_type: &str, content: &str) -> Value {
    let mut result = json!({
        "type": doc_type,
    });

    // Extract title
    if let Some(title_line) = content.lines().find(|l| l.starts_with("# ")) {
        result["title"] = json!(title_line.trim_start_matches("# ").trim());
    }

    // Extract path (content under ## Path section)
    if let Some(path) = extract_section_first_line(content, "Path") {
        result["path"] = json!(path);
    }

    // Extract template (```markdown ... ``` code block)
    if let Some(template) = extract_code_block(content, "markdown") {
        result["template"] = json!(template);
    }

    // Extract field description table
    if let Some(fields) = extract_table(content, "Field Description") {
        result["fields"] = fields;
    }

    // Extract sections list
    let sections = extract_h2_sections(content);
    if !sections.is_empty() {
        result["sections"] = json!(sections);
    }

    // Extract rules/notes list
    let mut rules = Vec::new();
    for section_name in &["Completion Rules", "Naming Rules", "Notes", "Additional Rules", "Cautions"] {
        if let Some(items) = extract_bullet_list(content, section_name) {
            for item in items {
                rules.push(json!({
                    "category": section_name,
                    "rule": item,
                }));
            }
        }
    }
    if !rules.is_empty() {
        result["rules"] = json!(rules);
    }

    // Required sections by mode (SPEC/PRD specific)
    if let Some(mode_table) = extract_table(content, "Required Sections by Mode") {
        result["mode_requirements"] = mode_table;
    }

    // Priority/severity definitions
    if let Some(prio_table) = extract_table(content, "Priority Definition") {
        result["priority_definitions"] = prio_table;
    }
    if let Some(complexity_table) = extract_table(content, "Complexity Definition") {
        result["complexity_definitions"] = complexity_table;
    }

    // create_command: Hint to create using dow doc (task/issue support -n)
    match doc_type {
        "task" => {
            result["create_command"] = json!("dow doc task -n <count>");
            result["create_hint"] = json!("Manual task file creation is prohibited, must use this command to generate template");
        }
        "issue" => {
            result["create_command"] = json!("dow doc issue -n <count> [--source test|devtest|audit|other]");
            result["create_hint"] = json!("Manual issue file creation is prohibited, must use this command to generate template");
        }
        "prd" | "spec" | "test" | "brainstorm" | "changelog" => {
            result["create_command"] = json!(format!("dow doc {}", doc_type));
            result["create_hint"] = json!(format!("Manual {}.md creation is prohibited, must use this command to generate", doc_type.to_uppercase()));
        }
        _ => {}
    }

    result
}

/// Extract the first non-empty line from a ## section
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

/// Extract code block for specified language
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

/// Extract table under a section as JSON array
fn extract_table(content: &str, heading: &str) -> Option<Value> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;

    // Find heading (supports ## and ### prefixes)
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed == heading {
            start = Some(i + 1);
            break;
        }
    }

    let start = start?;

    // Find table (lines starting with |)
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

    // Parse table headers
    let headers: Vec<String> = table_lines[0]
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // Skip separator row (second row), parse data rows
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
        rows.push(Value::Object(row));
    }

    Some(json!(rows))
}

/// Extract all ## section headings
fn extract_h2_sections(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| l.starts_with("## "))
        .map(|l| l.trim_start_matches("## ").trim().to_string())
        .collect()
}

/// Extract bullet list items under a section
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

// === Template creation functions (maintain original logic) ===

fn create_task(doc_root: &Path, count: u32) -> Result<(String, u32), DowError> {
    let task_dir = doc_root.join("task");
    fs::create_dir_all(&task_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let seq = next_seq(&task_dir, &format!("task_{}", today));
    let filename = format!("task_{}_{}.md", today, seq);
    let path = task_dir.join(&filename);

    let max_id = max_task_id_in_dir(&task_dir);

    let mut content = format!("---\ntitle: TASK - \nnums: {}\n---\n\n", count);

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] TASK-T{:03}: \n  - type: feat\n  - priority: P1\n  - refs: \n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - \n\n",
            max_id + i
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_issue(
    doc_root: &Path,
    count: u32,
    source: Option<&str>,
) -> Result<(String, u32), DowError> {
    let issue_dir = doc_root.join("issue");
    fs::create_dir_all(&issue_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let src = source.unwrap_or("other");
    let seq = next_seq(&issue_dir, &format!("issue_{}_{}", src, today));
    let filename = format!("issue_{}_{}_{}.md", src, today, seq);
    let path = issue_dir.join(&filename);

    let max_id = max_issue_id_in_dir(&issue_dir);

    let mut content = format!("---\nsource: {}\nnums: {}\n---\n\n", src, count);

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] ISSUE-I{:03}：\n  - severity: P1\n  - location：\n  - description：\n  - reproduce：\n  - fix：\n\n",
            max_id + i
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_single(
    doc_root: &Path,
    filename: &str,
    template: String,
) -> Result<(String, u32), DowError> {
    let path = doc_root.join(filename);
    if path.exists() {
        return Err(DowError::new(
            format!("{} already exists, will not overwrite", path.display()),
            1,
        ));
    }
    fs::write(&path, template).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), 1))
}

fn max_task_id_in_dir(task_dir: &Path) -> u32 {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("- [") {
                        // Match TASK-T001 or T001
                        if let Some(id) = extract_task_num(trimmed) {
                            max = max.max(id);
                        }
                    }
                }
            }
        }
    }
    max
}

fn max_issue_id_in_dir(issue_dir: &Path) -> u32 {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("- [") {
                        if let Some(id) = extract_issue_num(trimmed) {
                            max = max.max(id);
                        }
                    }
                }
            }
        }
    }
    max
}

fn extract_task_num(line: &str) -> Option<u32> {
    // Match "- [x] TASK-T001: ..." or "- [ ] TASK-T012: ..."
    let after = line.find("TASK-T").map(|p| &line[p + 6..])
        .or_else(|| line.find("] T").map(|p| &line[p + 3..]))?;
    let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

fn extract_issue_num(line: &str) -> Option<u32> {
    // Match "- [x] ISSUE-I001：..." or "- [ ] ISSUE-I012: ..."
    let after = line.find("ISSUE-I").map(|p| &line[p + 7..])
        .or_else(|| line.find("] I").map(|p| &line[p + 3..]))?;
    let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

fn next_seq(dir: &Path, prefix: &str) -> u32 {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(prefix) {
                if let Some(num_str) = name.strip_suffix(".md") {
                    if let Some(last) = num_str.rsplit('_').next() {
                        if let Ok(n) = last.parse::<u32>() {
                            max = max.max(n);
                        }
                    }
                }
            }
        }
    }
    max + 1
}

fn prd_template_inner(mode: &str, with_hints: bool) -> String {
    let sections = prd_sections_for_mode(mode);
    if sections.is_empty() {
        return format!("# Product Requirements Document (PRD)\n\n> {} mode skips PRD phase.\n", mode);
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
        if with_hints {
            match *sec {
                "Background & Motivation" => { out.push_str("<Why are we doing this>\n"); }
                "User Persona" => { out.push_str("<Who are the target users>\n"); }
                "User Flow" => { out.push_str("<How users will use it>\n"); }
                "Success Metrics" => { out.push_str("<How to measure success>\n"); }
                "Constraints & Assumptions" => { out.push_str("<Prerequisites and limitations>\n"); }
                "Open Questions" => { out.push_str("<Items to confirm>\n"); }
                _ => {}
            }
        }
        out.push('\n');
    }
    out
}

fn prd_template(mode: &str) -> String {
    prd_template_inner(mode, false)
}

fn prd_template_with_hints(mode: &str) -> String {
    prd_template_inner(mode, true)
}

/// Generate SPEC template. with_hints=true includes placeholder hints (for --md display), false is clean (for file creation)
fn spec_template_inner(mode: &str, with_hints: bool) -> String {
    let sections = spec_sections_for_mode(mode);
    let title_hint = if with_hints { " <topic>" } else { "" };
    let mut out = format!("# SPEC:{}\n\n", title_hint);

    for sec in &sections {
        out.push_str(&format!("## {}\n", sec));
        match *sec {
            "Goal" => {
                if with_hints { out.push_str("<objective>\n"); }
            }
            "Scope" => {
                out.push_str("### In\n### Out\n");
            }
            "Out of scope" => {
                if with_hints { out.push_str("<explicitly out-of-scope boundary>\n"); }
            }
            "Requirements Trace" => {
                out.push_str("| Req | AC | Notes |\n| --- | --- | --- |\n");
                if with_hints {
                    out.push_str("| PRD-FR-001 or user-request | SPEC-AC-001 | ADDED / MODIFIED / REMOVED |\n");
                }
            }
            "Design" => {
                if with_hints { out.push_str("<necessary design. keep it short.>\n"); }
            }
            "Acceptance" => {
                if with_hints {
                    out.push_str("- SPEC-AC-001: <testable acceptance criteria>\n- SPEC-AC-002: <testable acceptance criteria>\n");
                } else {
                    out.push_str("- SPEC-AC-001:\n- SPEC-AC-002:\n");
                }
            }
            "Risks" => {
                if with_hints { out.push_str("- <risks and fallback>\n"); }
            }
            "Test Plan" => {
                if with_hints { out.push_str("- <minimal validation approach>\n"); } else { out.push_str("- \n"); }
            }
            "Smoke Test" => {
                if with_hints { out.push_str("- <smoke test>\n"); } else { out.push_str("- \n"); }
            }
            _ => {}
        }
        out.push('\n');
    }

    // Self Check always included
    out.push_str("## Self Check\n");
    out.push_str("- [ ] Goal is clear\n");
    if sections.contains(&"Scope") || sections.contains(&"Out of scope") {
        out.push_str("- [ ] Scope is clear\n");
    }
    if sections.contains(&"Acceptance") {
        out.push_str("- [ ] Acceptance criteria are testable\n");
    }
    out.push_str("- [ ] Matches current mode\n");
    out
}

fn spec_template(mode: &str) -> String {
    spec_template_inner(mode, false)
}

fn spec_template_with_hints(mode: &str) -> String {
    spec_template_inner(mode, true)
}

fn test_template() -> String {
    r#"# Test Report

- Execution Time:
- Test Scope: Full / Specific Modules
- Total Cases:
- Passed:
- Failed:

## Failed Cases

| Module | Case | Error Message | Related Issue |
|--------|------|---------------|---------------|

## Passed Modules
"#
    .to_string()
}

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

fn changelog_template() -> String {
    "# Changelog\n".to_string()
}

// === Mode awareness features ===

/// Read current project mode (from STATUS.yaml), fallback to "full"
fn get_current_mode() -> String {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return "full".to_string();
    }
    let mode = yaml::get(&status_file, "mode").ok().flatten().unwrap_or_default();
    // audit/xxx → extract original mode
    if let Some(orig) = mode.strip_prefix("audit/") {
        orig.to_string()
    } else if mode.is_empty() {
        "full".to_string()
    } else {
        mode
    }
}

/// SPEC sections required for different modes
fn spec_sections_for_mode(mode: &str) -> Vec<&'static str> {
    match mode {
        "fast" => vec!["Goal", "Acceptance", "Test Plan"],
        "mvp" => vec!["Goal", "Out of scope", "Smoke Test"],
        "quick" => vec!["Goal", "Scope", "Design", "Acceptance", "Test Plan"],
        _ => vec!["Goal", "Scope", "Requirements Trace", "Design", "Acceptance", "Risks", "Test Plan"],
    }
}

/// PRD sections required for different modes
fn prd_sections_for_mode(mode: &str) -> Vec<&'static str> {
    match mode {
        "fast" | "mvp" => vec![],  // fast/mvp skip PRD
        "quick" => vec!["Background & Motivation", "Goals & Non-Goals", "Feature Requirements", "Success Metrics", "Open Questions"],
        _ => vec!["Background & Motivation", "Goals & Non-Goals", "User Persona", "Feature Requirements", "User Flow", "Success Metrics", "Constraints & Assumptions", "Open Questions"],
    }
}

/// Filter --md output by mode (handles "Required Sections by Mode" and template sections for spec/prd)
fn filter_md_by_mode(doc_type: &str, content: &str, mode: &str) -> String {
    if doc_type != "spec" && doc_type != "prd" {
        return content.to_string();
    }

    let mut result = Vec::new();
    let mut skip_mode_table = false;
    let mut skip_template_block = false;
    let mut in_code_fence = false;  // Track inside/outside code blocks

    for line in content.lines() {
        // Detect "Required Sections by Mode" section → replace with current mode list
        if !in_code_fence && line.starts_with("## Required Sections by Mode") {
            let sections = if doc_type == "spec" {
                spec_sections_for_mode(mode)
            } else {
                prd_sections_for_mode(mode)
            };
            if sections.is_empty() {
                // Current mode skips this phase, show full mode for reference
                result.push(format!("## Required Sections by Mode (current {} mode skips this phase)", mode));
                result.push(String::new());
                result.push("The following shows requirements for full mode:".to_string());
                result.push(String::new());
                let full_sections = if doc_type == "spec" {
                    spec_sections_for_mode("full")
                } else {
                    prd_sections_for_mode("full")
                };
                for s in &full_sections {
                    result.push(format!("- {}", s));
                }
            } else {
                result.push(format!("## Required Sections for Current Mode ({})", mode));
                result.push(String::new());
                for s in &sections {
                    result.push(format!("- {}", s));
                }
            }
            result.push(String::new());
            skip_mode_table = true;
            continue;
        }

        // Skip original mode table and degradation rules until next real ## section
        if skip_mode_table {
            if !in_code_fence && line.starts_with("## ") {
                skip_mode_table = false;
            } else {
                // Track code blocks
                if line.trim().starts_with("```") {
                    in_code_fence = !in_code_fence;
                }
                continue;
            }
        }

        // Detect template section → replace with current mode's dynamic template
        if !in_code_fence && line.starts_with("## Template") {
            let sections = if doc_type == "spec" {
                spec_sections_for_mode(mode)
            } else {
                prd_sections_for_mode(mode)
            };
            if sections.is_empty() {
                // Skipped phase: annotate and show full mode template for reference
                result.push(format!("## Template (current {} mode skips this phase, showing full mode reference)", mode));
            } else {
                result.push("## Template".to_string());
            }
            result.push(String::new());
            result.push("```markdown".to_string());
            let effective_mode = if sections.is_empty() { "full" } else { mode };
            let template = if doc_type == "spec" {
                spec_template_with_hints(effective_mode)
            } else {
                prd_template_with_hints(effective_mode)
            };
            result.push(template.trim_end().to_string());
            result.push("```".to_string());
            result.push(String::new());
            skip_template_block = true;
            continue;
        }

        // Skip original template code block (need to track ``` ending)
        if skip_template_block {
            if line.trim().starts_with("```") {
                in_code_fence = !in_code_fence;
                // After code block ends, wait for next ## section
                if !in_code_fence {
                    continue;
                }
            }
            if in_code_fence {
                continue;
            }
            // Code block already ended, resume normal when encountering next ## section
            if line.starts_with("## ") {
                skip_template_block = false;
                result.push(line.to_string());
            }
            continue;
        }

        // Normal line: track code block state
        if line.trim().starts_with("```") {
            in_code_fence = !in_code_fence;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

// === Persistent documentation initialization ===

const DEFAULT_DOCS: &[&str] = &["docs/structure.md", "docs/decisions.md", "docs/usage.md"];

fn run_doc_init(project_name: Option<&str>, human: bool) -> Result<i32, DowError> {
    let project_root = doc_root::project_root();
    let name = project_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| infer_project_name(&project_root));

    let docs_dir = project_root.join("docs");
    fs::create_dir_all(&docs_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let mut created: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    // README.md
    let readme_path = project_root.join("README.md");
    if !readme_path.exists() {
        let content = format!(
            "# {}\n\n<One-line description>\n\n## Quick Start\n\n<Installation and basic usage>\n\n## Documentation\n\n- [Project Structure](docs/structure.md)\n- [Design Decisions](docs/decisions.md)\n- [Usage Guide](docs/usage.md)\n",
            name
        );
        fs::write(&readme_path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
        created.push("README.md".to_string());
    } else {
        skipped.push("README.md".to_string());
    }

    // docs/structure.md
    let structure_path = docs_dir.join("structure.md");
    if !structure_path.exists() {
        fs::write(
            &structure_path,
            "# Project Structure\n\n## Directory Tree\n\n<To be filled>\n\n## Module Responsibilities\n\n<To be filled>\n",
        )
        .map_err(|e| DowError::new(e.to_string(), 1))?;
        created.push("docs/structure.md".to_string());
    } else {
        skipped.push("docs/structure.md".to_string());
    }

    // docs/decisions.md
    let decisions_path = docs_dir.join("decisions.md");
    if !decisions_path.exists() {
        fs::write(
            &decisions_path,
            "# Design Decision Records\n\n## <Decision Title>\n\n- **Date**: YYYY-MM-DD\n- **Decision**: <what>\n- **Rationale**: <why>\n- **Consequences**: <consequence>\n",
        )
        .map_err(|e| DowError::new(e.to_string(), 1))?;
        created.push("docs/decisions.md".to_string());
    } else {
        skipped.push("docs/decisions.md".to_string());
    }

    // docs/usage.md
    let usage_path = docs_dir.join("usage.md");
    if !usage_path.exists() {
        fs::write(
            &usage_path,
            "# Usage Guide\n\n## Development Environment\n\n<To be filled>\n\n## Common Tasks\n\n<To be filled>\n",
        )
        .map_err(|e| DowError::new(e.to_string(), 1))?;
        created.push("docs/usage.md".to_string());
    } else {
        skipped.push("docs/usage.md".to_string());
    }

    // Update STATUS.yaml docs field
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");
    if status_file.exists() {
        let existing = yaml::get_list(&status_file, "docs")
            .unwrap_or_default();
        if existing.is_empty() {
            let docs_list: Vec<String> = DEFAULT_DOCS.iter().map(|s| s.to_string()).collect();
            yaml::set_list(&status_file, "docs", &docs_list)
                .map_err(|e| DowError::new(e.to_string(), 1))?;
        }
    }

    if human {
        println!("[dev-flow] Persistent documentation initialization completed");
        if !created.is_empty() {
            println!("  Created: {}", created.join(", "));
        }
        if !skipped.is_empty() {
            println!("  Skipped (already exists): {}", skipped.join(", "));
        }
        println!("  STATUS.yaml docs field updated");
    } else {
        output::print_json(&json!({
            "created": created,
            "skipped": skipped,
            "docs_registered": DEFAULT_DOCS,
        }));
    }

    Ok(0)
}

fn infer_project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".to_string())
}

// === doc check-sync ===

fn run_doc_check_sync(since: Option<&str>, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");

    let mut docs = yaml::get_list(&status_file, "docs").unwrap_or_default();
    // README.md implicitly checked
    docs.push("README.md".to_string());

    let project_root = doc_root::project_root();
    let ref_valid = since.map_or(false, |r| git_ref_exists(r));

    let mut synced: Vec<String> = Vec::new();
    let mut outdated: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for doc in &docs {
        let full_path = project_root.join(doc);
        if !full_path.exists() {
            missing.push(doc.clone());
            continue;
        }
        if let Some(git_ref) = since {
            if ref_valid {
                if file_changed_since(&project_root, doc, git_ref) {
                    synced.push(doc.clone());
                } else {
                    outdated.push(doc.clone());
                }
            } else {
                // ref invalid, fallback to file existence
                synced.push(doc.clone());
            }
        } else {
            // No --since, only check file existence
            synced.push(doc.clone());
        }
    }

    if human {
        if !outdated.is_empty() {
            println!("[dev-flow] The following documents have not been updated since {}:", since.unwrap_or("?"));
            for d in &outdated {
                println!("  - {}", d);
            }
        }
        if !missing.is_empty() {
            println!("[dev-flow] The following registered documents do not exist:");
            for d in &missing {
                println!("  - {}", d);
            }
        }
        if outdated.is_empty() && missing.is_empty() {
            println!("[dev-flow] All persistent documents are synced");
        }
    } else {
        output::print_json(&json!({
            "synced": synced,
            "outdated": outdated,
            "missing": missing,
            "since": since.unwrap_or(""),
            "ref_valid": ref_valid,
        }));
    }

    Ok(0)
}

fn git_ref_exists(git_ref: &str) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--verify", git_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn file_changed_since(project_root: &Path, file: &str, git_ref: &str) -> bool {
    let output = std::process::Command::new("git")
        .args(["log", &format!("{}..HEAD", git_ref), "--", file])
        .current_dir(project_root)
        .output();
    match output {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => false,
    }
}

// === doc list ===

fn run_doc_list(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");
    let docs = yaml::get_list(&status_file, "docs").unwrap_or_default();
    let project_root = doc_root::project_root();

    #[derive(Serialize)]
    struct DocEntry {
        path: String,
        exists: bool,
        last_modified: Option<String>,
    }

    let entries: Vec<DocEntry> = docs.iter().map(|d| {
        let full_path = project_root.join(d);
        let exists = full_path.exists();
        let last_modified = if exists {
            git_last_modified(&project_root, d)
        } else {
            None
        };
        DocEntry { path: d.clone(), exists, last_modified }
    }).collect();

    if human {
        if entries.is_empty() {
            println!("[dev-flow] No registered documents");
        } else {
            println!("[dev-flow] Registered documents list:");
            for e in &entries {
                let status = if e.exists {
                    e.last_modified.as_deref().unwrap_or("not tracked")
                } else {
                    "does not exist"
                };
                println!("  {} ({})", e.path, status);
            }
        }
    } else {
        output::print_json(&entries);
    }

    Ok(0)
}

fn git_last_modified(project_root: &Path, file: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ci", "--", file])
        .current_dir(project_root)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Filter mode_requirements and template in --json output by mode
fn filter_json_by_mode(parsed: &mut Value, mode: &str) {
    let doc_type = parsed.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // mode_requirements: only keep rows marked for current mode
    if let Some(arr) = parsed.get("mode_requirements").and_then(|v| v.as_array()) {
        let filtered: Vec<Value> = arr.iter().filter(|row| {
            if let Some(val) = row.get(mode).and_then(|v| v.as_str()) {
                val == "✓" || !val.contains('—')
            } else {
                true
            }
        }).cloned().collect();
        parsed["mode_requirements"] = json!(filtered);
    }

    // template: replace with current mode's dynamic template (with placeholder hints)
    if doc_type == "spec" || doc_type == "prd" {
        let template = if doc_type == "spec" {
            spec_template_with_hints(mode)
        } else {
            prd_template_with_hints(mode)
        };
        parsed["template"] = json!(template.trim_end());
    }

    parsed["current_mode"] = json!(mode);
}
