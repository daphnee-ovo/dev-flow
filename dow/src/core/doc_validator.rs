// dow/src/core/
// ├── doc_validator.rs  -- .dev-doc file validity validation
//    Embed specifications from references/.dev-doc/*.md at compile time, parse and validate files at runtime
//
// Related Docs:
// - [ISSUE Specification](../../../references/.dev-doc/ISSUE.md)
// - [TASK Specification](../../../references/.dev-doc/TASK-FILE.md)

use std::fs;
use std::path::Path;

// Embed specification md at compile time
const REF_ISSUE: &str = include_str!("../../references/.dev-doc/ISSUE.md");
const REF_TASK: &str = include_str!("../../references/.dev-doc/TASK-FILE.md");

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub file: String,
    pub kind: ErrorKind,
    pub message: String,
    /// Whether it can be auto-fixed by dow fix
    pub fixable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    BadFilename,
    MissingFrontmatter,
    InvalidFrontmatter,
    MissingRequiredField,
    InvalidFieldValue,
}

/// Validation rules parsed from ISSUE.md specification
struct IssueSpec {
    valid_sources: Vec<String>,
    valid_severities: Vec<String>,
}

/// Validation rules parsed from TASK-FILE.md specification
struct TaskSpec {
    valid_priorities: Vec<String>,
    valid_complexities: Vec<String>,
    required_fields: Vec<String>,
}

/// Parse ISSUE.md specification
fn parse_issue_spec() -> IssueSpec {
    let mut valid_sources = Vec::new();
    let mut valid_severities = Vec::new();

    for line in REF_ISSUE.lines() {
        // Extract enum values from field description table
        // Format: | source | `test` / `devtest` / `other` / `audit` | ... |
        if line.contains("| source") || line.contains("| source") {
            valid_sources = extract_enum_values(line);
        }
        if line.contains("| severity") {
            valid_severities = extract_enum_values(line);
        }
    }

    // fallback
    if valid_sources.is_empty() {
        valid_sources = vec!["test".into(), "devtest".into(), "other".into(), "audit".into()];
    }
    if valid_severities.is_empty() {
        valid_severities = vec!["P0".into(), "P1".into(), "P2".into()];
    }

    IssueSpec { valid_sources, valid_severities }
}

/// Parse TASK-FILE.md specification
fn parse_task_spec() -> TaskSpec {
    let mut valid_priorities = Vec::new();
    let mut valid_complexities = Vec::new();
    let mut required_fields = Vec::new();

    let mut in_fields_table = false;

    for line in REF_TASK.lines() {
        // Detect field description table region
        if line.starts_with("## Field Description") {
            in_fields_table = true;
            continue;
        }
        if in_fields_table && line.starts_with("## ") {
            in_fields_table = false;
        }

        // priority enum (from table or Priority definition section)
        if line.contains("| priority") && line.contains("P0") {
            valid_priorities = extract_enum_values(line);
        }
        // complexity enum
        if line.contains("| complexity") || (line.contains("| `S`") && line.contains("small task")) {
            if valid_complexities.is_empty() {
                valid_complexities = extract_complexity_values(REF_TASK);
            }
        }
        // done_when is required (specification says "must be objective and concrete")
        if in_fields_table && line.contains("| done_when") {
            required_fields.push("done_when".into());
        }
        if in_fields_table && line.contains("| priority") {
            required_fields.push("priority".into());
        }
    }

    // fallback
    if valid_priorities.is_empty() {
        valid_priorities = vec!["P0".into(), "P1".into(), "P2".into()];
    }
    if valid_complexities.is_empty() {
        valid_complexities = vec!["S".into(), "M".into(), "L".into()];
    }
    if required_fields.is_empty() {
        required_fields = vec!["priority".into(), "done_when".into()];
    }

    TaskSpec { valid_priorities, valid_complexities, required_fields }
}

/// Extract enum values in `value` / `value` format from md table row
fn extract_enum_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    // Match `xxx` format
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            let mut val = String::new();
            for inner in chars.by_ref() {
                if inner == '`' {
                    break;
                }
                val.push(inner);
            }
            if !val.is_empty() && !val.contains(' ') && !val.contains('<') {
                values.push(val);
            }
        }
    }
    values
}

/// Extract complexity enum values from TASK-FILE.md (Complexity definition table)
fn extract_complexity_values(content: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_section = false;
    for line in content.lines() {
        if line.starts_with("## Complexity") {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if in_section && line.starts_with("| `") {
            // | `S` | small task | ...
            if let Some(val) = extract_first_backtick_value(line) {
                values.push(val);
            }
        }
    }
    values
}

fn extract_first_backtick_value(line: &str) -> Option<String> {
    let start = line.find('`')? + 1;
    let end = line[start..].find('`')? + start;
    let val = &line[start..end];
    if val.is_empty() { None } else { Some(val.to_string()) }
}

// ==================== Validation Logic ====================

/// Validate single issue file
pub fn validate_issue_file(path: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let spec = parse_issue_spec();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // 1. Filename validation
    if let Some(e) = validate_issue_filename(&filename, &spec) {
        errors.push(e);
    }

    // 2. Content validation
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    errors.extend(validate_issue_content(&filename, &content, &spec));
    errors
}

/// Validate single task file
pub fn validate_task_file(path: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let spec = parse_task_spec();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // 1. Filename validation
    if let Some(e) = validate_task_filename(&filename) {
        errors.push(e);
    }

    // 2. Content validation
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    // doc_root = parent of task file's parent (task/)
    let doc_root = path.parent().and_then(|p| p.parent());
    errors.extend(validate_task_content(&filename, &content, &spec, doc_root));
    errors
}

/// Validate issue filename
/// Valid format: issue_<source>_<YYYY-MM-DD>_<seq>.md or closed_issue_<source>_<YYYY-MM-DD>_<seq>.md
fn validate_issue_filename(filename: &str, spec: &IssueSpec) -> Option<ValidationError> {
    let stem = filename.strip_suffix(".md")?;
    let parts: Vec<&str> = if stem.starts_with("closed_issue_") {
        stem["closed_issue_".len()..].splitn(4, '_').collect()
    } else if stem.starts_with("issue_") {
        stem["issue_".len()..].splitn(4, '_').collect()
    } else {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "Filename does not conform to specification (should be issue_<source>_<YYYY-MM-DD>_<seq>.md)".into(),
            fixable: false,
        });
    };

    // Need at least source + date + seq = parts obtained from splitn(4, '_')
    // Actual: source_YYYY-MM-DD_seq → splitn(4, '_') → [source, YYYY-MM-DD, seq]
    // issue_test_2026-05-29_1.md → after removing prefix → "test_2026-05-29_1"
    // splitn(4, '_') → ["test", "2026-05-29", "1"]
    if parts.len() < 3 {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "Filename missing required parts (need source, date, sequence number)".into(),
            fixable: false,
        });
    }

    let source = parts[0];
    if !spec.valid_sources.iter().any(|s| s == source) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!(
                "source '{}' is invalid, valid values: {}",
                source,
                spec.valid_sources.join("/")
            ),
            fixable: false,
        });
    }

    // Validate date format YYYY-MM-DD
    let date = parts[1];
    if !is_valid_date(date) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!("Date '{}' format is invalid (need YYYY-MM-DD)", date),
            fixable: false,
        });
    }

    // Validate sequence number is numeric
    let seq = parts[2];
    if seq.parse::<u32>().is_err() {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!("Sequence number '{}' is not a valid number", seq),
            fixable: false,
        });
    }

    None
}

/// Validate task filename
/// Valid format: task_<YYYY-MM-DD>_<seq>.md or done_task_<YYYY-MM-DD>_<seq>.md
fn validate_task_filename(filename: &str) -> Option<ValidationError> {
    let stem = filename.strip_suffix(".md")?;
    let name_part = if stem.starts_with("done_task_") {
        &stem["done_task_".len()..]
    } else if stem.starts_with("task_") {
        &stem["task_".len()..]
    } else {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "Invalid filename (expected task_<YYYY-MM-DD>_<seq>.md)".into(),
            fixable: false,
        });
    };

    // name_part should be "YYYY-MM-DD_seq", split at last underscore
    if let Some(last_underscore) = name_part.rfind('_') {
        let date = &name_part[..last_underscore];
        let seq = &name_part[last_underscore + 1..];

        if !is_valid_date(date) {
            return Some(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::BadFilename,
                message: format!("date '{}' is invalid (expected YYYY-MM-DD)", date),
                fixable: false,
            });
        }
        if seq.parse::<u32>().is_err() {
            return Some(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::BadFilename,
                message: format!("sequence '{}' is not a valid number", seq),
                fixable: false,
            });
        }
    } else {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "Filename missing sequence number".into(),
            fixable: false,
        });
    }

    None
}

/// Validate issue file content
fn validate_issue_content(filename: &str, content: &str, spec: &IssueSpec) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check YAML frontmatter
    if !content.starts_with("---") {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingFrontmatter,
            message: "Missing YAML frontmatter (---)".into(),
            fixable: true,
        });
    } else if content[3..].find("---").is_none() {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFrontmatter,
            message: "Malformed YAML frontmatter: opening `---` has no closing `---`".into(),
            fixable: false,
        });
    } else {
        let fm = extract_frontmatter(content);
        // source field
        if let Some(source_val) = extract_fm_value(&fm, "source") {
            if !spec.valid_sources.contains(&source_val) {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "frontmatter source '{}' is invalid, valid values: {}",
                        source_val,
                        spec.valid_sources.join("/")
                    ),
                    fixable: false,
                });
            }
        } else {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter missing source field".into(),
                fixable: true,
            });
        }
        // nums field
        if extract_fm_value(&fm, "nums").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter missing nums field".into(),
                fixable: true,
            });
        }
    }

    // Check severity and sequence for each issue item
    let mut in_item = false;
    let mut item_title = String::new();
    let mut has_severity = false;
    let mut expected_issue_seq = 1u32;

    for line in content.lines() {
        if line.starts_with("- [ ]") || line.starts_with("- [x]") {
            // previous item missing severity
            if in_item && !has_severity {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::MissingRequiredField,
                    message: format!("issue item '{}' missing severity field", item_title),
                    fixable: false,
                });
            }
            in_item = true;
            item_title = line[5..].trim().to_string();
            has_severity = false;
            // Validate issue sequence: ISSUE-I + digits
            if let Some(e) = validate_issue_item_seq(&item_title, expected_issue_seq, filename) {
                errors.push(e);
            }
            expected_issue_seq += 1;
        } else if in_item && line.contains("severity:") {
            has_severity = true;
            let val = line.split("severity:").nth(1).unwrap_or("").trim();
            if !spec.valid_severities.iter().any(|s| s == val) {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "severity '{}' is invalid, valid values: {}",
                        val,
                        spec.valid_severities.join("/")
                    ),
                    fixable: false,
                });
            }
        }
    }
    // last item
    if in_item && !has_severity {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingRequiredField,
            message: format!("issue item '{}' missing severity field", item_title),
            fixable: false,
        });
    }

    errors
}

/// Validate task file content
fn validate_task_content(filename: &str, content: &str, spec: &TaskSpec, doc_root: Option<&Path>) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check YAML frontmatter
    if !content.starts_with("---") {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingFrontmatter,
            message: "Missing YAML frontmatter (---)".into(),
            fixable: true,
        });
    } else if content[3..].find("---").is_none() {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFrontmatter,
            message: "Malformed YAML frontmatter: opening `---` has no closing `---`".into(),
            fixable: false,
        });
    } else {
        let fm = extract_frontmatter(content);
        if extract_fm_value(&fm, "title").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter missing title field".into(),
                fixable: true,
            });
        }
        if extract_fm_value(&fm, "nums").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter missing nums field".into(),
                fixable: true,
            });
        }
    }

    // Check required fields and sequence for each task item
    let mut in_item = false;
    let mut item_title = String::new();
    let mut found_fields: Vec<String> = Vec::new();
    let mut expected_task_seq = 1u32;
    let mut task_refs: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.starts_with("- [ ]") || line.starts_with("- [x]") {
            // check previous item
            check_task_item_fields(filename, &item_title, &found_fields, spec, &mut errors);
            in_item = true;
            item_title = line[5..].trim().to_string();
            found_fields.clear();
            // Validate task sequence: TASK-T + digits
            if let Some(e) = validate_task_item_seq(&item_title, expected_task_seq, filename) {
                errors.push(e);
            }
            expected_task_seq += 1;
        } else if in_item {
            // collect fields
            let trimmed = line.trim_start();
            if trimmed.starts_with("- priority:") {
                found_fields.push("priority".into());
                let val = trimmed.split("priority:").nth(1).unwrap_or("").trim();
                if !spec.valid_priorities.iter().any(|p| p == val) {
                    errors.push(ValidationError {
                        file: filename.to_string(),
                        kind: ErrorKind::InvalidFieldValue,
                        message: format!(
                            "task '{}' priority '{}' is invalid, valid values: {}",
                            item_title, val, spec.valid_priorities.join("/")
                        ),
                        fixable: false,
                    });
                }
            } else if trimmed.starts_with("- complexity:") {
                found_fields.push("complexity".into());
                let val = trimmed.split("complexity:").nth(1).unwrap_or("").trim();
                if !spec.valid_complexities.iter().any(|c| c == val) {
                    errors.push(ValidationError {
                        file: filename.to_string(),
                        kind: ErrorKind::InvalidFieldValue,
                        message: format!(
                            "task '{}' complexity '{}' is invalid, valid values: {}",
                            item_title, val, spec.valid_complexities.join("/")
                        ),
                        fixable: false,
                    });
                }
            } else if trimmed.starts_with("- done_when:") {
                found_fields.push("done_when".into());
            } else if trimmed.starts_with("- refs:") {
                let val = trimmed.split("refs:").nth(1).unwrap_or("").trim();
                // Collect SPEC-AC references
                for part in val.split(',') {
                    let r = part.trim();
                    if r.starts_with("SPEC-AC-") {
                        task_refs.push(r.to_string());
                    }
                }
            }
        }
    }
    // Last item
    check_task_item_fields(filename, &item_title, &found_fields, spec, &mut errors);

    // Validate that SPEC-AC refs exist in SPEC.md
    if !task_refs.is_empty() {
        if let Some(root) = doc_root {
            let spec_path = root.join("SPEC.md");
            if spec_path.exists() {
                let spec_acs = extract_spec_acs_from_file(&spec_path);
                for r in &task_refs {
                    if !spec_acs.contains(r) {
                        errors.push(ValidationError {
                            file: filename.to_string(),
                            kind: ErrorKind::InvalidFieldValue,
                            message: format!("refs reference '{}' does not exist in SPEC.md", r),
                            fixable: false,
                        });
                    }
                }
            }
        }
    }

    errors
}

fn check_task_item_fields(
    filename: &str,
    item_title: &str,
    found_fields: &[String],
    spec: &TaskSpec,
    errors: &mut Vec<ValidationError>,
) {
    if item_title.is_empty() {
        return;
    }
    for req in &spec.required_fields {
        if !found_fields.iter().any(|f| f == req) {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: format!("task '{}' missing required field '{}'", item_title, req),
                fixable: false,
            });
        }
    }
}

// ==================== Batch validation ====================

/// Validate all issue files in directory
pub fn validate_all_issues(doc_root: &Path) -> Vec<ValidationError> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return vec![];
    }

    let mut all_errors = Vec::new();
    let mut all_issue_ids: Vec<(u32, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("issue_") || name.starts_with("closed_issue_"))
            {
                all_errors.extend(validate_issue_file(&entry.path()));
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [") {
                            let title = line[5..].trim();
                            if let Some(num) = extract_issue_id_num(title) {
                                all_issue_ids.push((num, name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Global sequence continuity check
    if !all_issue_ids.is_empty() {
        all_issue_ids.sort_by_key(|(num, _)| *num);

        for i in 1..all_issue_ids.len() {
            if all_issue_ids[i].0 == all_issue_ids[i - 1].0 {
                all_errors.push(ValidationError {
                    file: all_issue_ids[i].1.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "Duplicate issue sequence: ISSUE-I{:03} found in {} and {}",
                        all_issue_ids[i].0, all_issue_ids[i - 1].1, all_issue_ids[i].1
                    ),
                    fixable: false,
                });
            }
        }

        for (idx, (num, filename)) in all_issue_ids.iter().enumerate() {
            let expected = idx as u32 + 1;
            if *num != expected {
                all_errors.push(ValidationError {
                    file: filename.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "Issue global sequence not continuous: expected ISSUE-I{:03}, got ISSUE-I{:03}",
                        expected, num
                    ),
                    fixable: false,
                });
                break;
            }
        }
    }

    all_errors
}

fn extract_issue_id_num(title: &str) -> Option<u32> {
    let prefix = "ISSUE-I";
    if !title.starts_with(prefix) {
        return None;
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

/// Validate all task files in directory
pub fn validate_all_tasks(doc_root: &Path) -> Vec<ValidationError> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return vec![];
    }

    let mut all_errors = Vec::new();
    let mut all_task_ids: Vec<(u32, String)> = Vec::new(); // (id_num, filename)

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("task_") || name.starts_with("done_task_"))
            {
                all_errors.extend(validate_task_file(&entry.path()));
                // collect all task IDs
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if line.starts_with("- [") {
                            let title = line[5..].trim();
                            if let Some(num) = extract_task_id_num(title) {
                                all_task_ids.push((num, name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Global sequence continuity check: verify 1..N after sort
    if !all_task_ids.is_empty() {
        all_task_ids.sort_by_key(|(num, _)| *num);

        // check duplicates
        for i in 1..all_task_ids.len() {
            if all_task_ids[i].0 == all_task_ids[i - 1].0 {
                all_errors.push(ValidationError {
                    file: all_task_ids[i].1.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "Duplicate task sequence: TASK-T{:03} found in {} and {}",
                        all_task_ids[i].0, all_task_ids[i - 1].1, all_task_ids[i].1
                    ),
                    fixable: false,
                });
            }
        }

        // check continuity: from 1 to max
        for (idx, (num, filename)) in all_task_ids.iter().enumerate() {
            let expected = idx as u32 + 1;
            if *num != expected {
                all_errors.push(ValidationError {
                    file: filename.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "Task global sequence not continuous: expected TASK-T{:03}, got TASK-T{:03}",
                        expected, num
                    ),
                    fixable: false,
                });
                break; // report first gap only
            }
        }
    }

    all_errors
}

fn extract_task_id_num(title: &str) -> Option<u32> {
    let prefix = "TASK-T";
    if !title.starts_with(prefix) {
        return None;
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

/// Validate SPEC-AC sequence is monotonically increasing
pub fn validate_spec(doc_root: &Path) -> Vec<ValidationError> {
    let spec_file = doc_root.join("SPEC.md");
    if !spec_file.exists() {
        return vec![];
    }

    let content = match fs::read_to_string(&spec_file) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut errors = Vec::new();
    let mut expected_seq = 1u32;

    for line in content.lines() {
        let trimmed = line.trim_start_matches("- ").trim();
        if let Some(rest) = trimmed.strip_prefix("SPEC-AC-") {
            // extract sequence number
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if num_str.is_empty() {
                errors.push(ValidationError {
                    file: "SPEC.md".into(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!("SPEC-AC sequence format invalid: SPEC-AC-{}", rest.chars().take(10).collect::<String>()),
                    fixable: false,
                });
                continue;
            }
            if let Ok(actual) = num_str.parse::<u32>() {
                if actual != expected_seq {
                    errors.push(ValidationError {
                        file: "SPEC.md".into(),
                        kind: ErrorKind::InvalidFieldValue,
                        message: format!(
                            "SPEC-AC sequence not continuous: expected SPEC-AC-{:03}, got SPEC-AC-{:03}",
                            expected_seq, actual
                        ),
                        fixable: false,
                    });
                }
                expected_seq = actual + 1;
            }
        }
    }

    errors
}

/// Validate all .dev-doc files
/// Includes: STATUS.yaml, task/issue content, SPEC sequence, issue consistency, illegal files
pub fn validate_all(doc_root: &Path) -> Vec<ValidationError> {
    let mut errors = validate_status_yaml(doc_root);
    errors.extend(validate_all_issues(doc_root));
    errors.extend(validate_all_tasks(doc_root));
    errors.extend(validate_spec(doc_root));
    errors.extend(validate_issue_consistency(doc_root));
    errors.extend(validate_no_illegal_files(doc_root));
    errors
}

/// Validate STATUS.yaml required fields and enum values
fn validate_status_yaml(doc_root: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let status_file = doc_root.join("STATUS.yaml");
    if !status_file.exists() {
        errors.push(ValidationError {
            file: "STATUS.yaml".into(),
            kind: ErrorKind::MissingRequiredField,
            message: "STATUS.yaml does not exist".into(),
            fixable: false,
        });
        return errors;
    }

    let content = match fs::read_to_string(&status_file) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    let mut map = std::collections::BTreeMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            map.insert(key, value);
        }
    }

    for field in &["name", "phase", "mode", "updated", "started"] {
        if !map.contains_key(*field) || map[*field].is_empty() {
            errors.push(ValidationError {
                file: "STATUS.yaml".into(),
                kind: ErrorKind::MissingRequiredField,
                message: format!("Missing required field: {}", field),
                fixable: false,
            });
        }
    }

    if let Some(phase) = map.get("phase") {
        let valid = ["PRD", "SPEC", "TASK", "DEV", "TEST", "DONE"];
        if !phase.is_empty() && !valid.contains(&phase.as_str()) {
            errors.push(ValidationError {
                file: "STATUS.yaml".into(),
                kind: ErrorKind::InvalidFieldValue,
                message: format!("phase '{}' is invalid (valid values: {})", phase, valid.join("/")),
                fixable: false,
            });
        }
    }

    if let Some(mode) = map.get("mode") {
        let valid_pattern = mode == "full"
            || mode == "quick"
            || mode == "fast"
            || mode == "mvp"
            || mode.starts_with("audit/");
        if !mode.is_empty() && !valid_pattern {
            errors.push(ValidationError {
                file: "STATUS.yaml".into(),
                kind: ErrorKind::InvalidFieldValue,
                message: format!("mode '{}' is invalid (valid values: full/quick/fast/mvp/audit/*)", mode),
                fixable: false,
            });
        }
    }

    errors
}

/// Validate issue file status consistency (checkbox vs closed_ prefix)
fn validate_issue_consistency(doc_root: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return errors;
    }

    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            let content = match fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let total: usize = content.lines().filter(|l| l.starts_with("- [")).count();
            let done: usize = content.lines().filter(|l| l.starts_with("- [x]")).count();
            if total > 0 && total == done && !name.starts_with("closed_") {
                errors.push(ValidationError {
                    file: name.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: "All issues checked but file not renamed with closed_ prefix".into(),
                    fixable: true,
                });
            }
            if name.starts_with("closed_") && total > 0 && done < total {
                errors.push(ValidationError {
                    file: name.clone(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: "File has closed_ prefix but contains unchecked issue items".into(),
                    fixable: false,
                });
            }
        }
    }

    errors
}

/// Detect non-workflow files in .dev-doc/<branch>/
fn validate_no_illegal_files(doc_root: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let valid_top_files = [
        "PRD.md", "SPEC.md", "TEST.md", "BRAINSTORM.md", "CHANGELOG.md", "STATUS.yaml", "claim.lock",
    ];
    let valid_subdirs = ["task", "issue"];

    let ignored = build_ignore_set(doc_root);

    if let Ok(entries) = fs::read_dir(doc_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if should_ignore(&name, &ignored) {
                continue;
            }
            let path = entry.path();

            if path.is_file() {
                if !valid_top_files.contains(&name.as_str()) {
                    errors.push(ValidationError {
                        file: name,
                        kind: ErrorKind::BadFilename,
                        message: "Non-workflow file not allowed in .dev-doc (valid: PRD.md, SPEC.md, TEST.md, BRAINSTORM.md, CHANGELOG.md, STATUS.yaml)".into(),
                        fixable: false,
                    });
                }
            } else if path.is_dir() {
                if !valid_subdirs.contains(&name.as_str()) {
                    errors.push(ValidationError {
                        file: name,
                        kind: ErrorKind::BadFilename,
                        message: "Non-workflow directory not allowed in .dev-doc (valid: task/, issue/)".into(),
                        fixable: false,
                    });
                }
            }
        }
    }

    // Check task/ file naming
    let task_dir = doc_root.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if should_ignore(&name, &ignored) {
                    continue;
                }
                if entry.path().is_file() {
                    if !((name.starts_with("task_") || name.starts_with("done_task_")) && name.ends_with(".md")) {
                        errors.push(ValidationError {
                            file: format!("task/{}", name),
                            kind: ErrorKind::BadFilename,
                            message: "Only task_*.md or done_task_*.md files allowed in task/".into(),
                            fixable: false,
                        });
                    }
                }
            }
        }
    }

    // Check issue/ file naming
    let issue_dir = doc_root.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if should_ignore(&name, &ignored) {
                    continue;
                }
                if entry.path().is_file() {
                    if !((name.starts_with("issue_") || name.starts_with("closed_issue_")) && name.ends_with(".md")) {
                        errors.push(ValidationError {
                            file: format!("issue/{}", name),
                            kind: ErrorKind::BadFilename,
                            message: "Only issue_*.md or closed_issue_*.md files allowed in issue/".into(),
                            fixable: false,
                        });
                    }
                }
            }
        }
    }

    errors
}

/// Format errors as human-readable output
pub fn format_errors_human(errors: &[ValidationError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&format!("[dev-flow] Document validation failed ({} errors):\n", errors.len()));
    for e in errors {
        let fixable_hint = if e.fixable { " [fixable]" } else { "" };
        out.push_str(&format!("  - {}：{}{}\n", e.file, e.message, fixable_hint));
    }
    out.push_str("\nHint: run `dow fix` to auto-fix fixable issues.\n");
    out
}

// ==================== Utility functions ====================

fn is_valid_date(s: &str) -> bool {
    // YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

fn extract_frontmatter(content: &str) -> String {
    if !content.starts_with("---") {
        return String::new();
    }
    if let Some(end) = content[3..].find("---") {
        content[3..3 + end].to_string()
    } else {
        String::new()
    }
}

fn extract_fm_value(fm: &str, key: &str) -> Option<String> {
    for line in fm.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{}:", key)) {
            let val = rest.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Validate issue item sequence format (continuity checked globally)
fn validate_issue_item_seq(title: &str, _expected: u32, filename: &str) -> Option<ValidationError> {
    let prefix = "ISSUE-I";
    if !title.starts_with(prefix) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFieldValue,
            message: format!(
                "Invalid issue item sequence (expected ISSUE-I00N: prefix): '{}'",
                title.chars().take(30).collect::<String>()
            ),
            fixable: false,
        });
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFieldValue,
            message: format!("Issue item sequence missing number: '{}'", title.chars().take(30).collect::<String>()),
            fixable: false,
        });
    }
    None
}

/// Validate task item sequence format (continuity checked globally)
fn validate_task_item_seq(title: &str, _expected: u32, filename: &str) -> Option<ValidationError> {
    let prefix = "TASK-T";
    if !title.starts_with(prefix) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFieldValue,
            message: format!(
                "Invalid task item sequence (expected TASK-T00N: prefix): '{}'",
                title.chars().take(30).collect::<String>()
            ),
            fixable: false,
        });
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::InvalidFieldValue,
            message: format!("Task item sequence missing number: '{}'", title.chars().take(30).collect::<String>()),
            fixable: false,
        });
    }
    None
}

/// Extract all defined SPEC-AC-xxx identifiers from SPEC.md
fn extract_spec_acs_from_file(spec_path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(spec_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut acs = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start_matches("- ").trim();
        if let Some(rest) = trimmed.strip_prefix("SPEC-AC-") {
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !num_str.is_empty() {
                acs.push(format!("SPEC-AC-{}", num_str));
            }
        }
    }
    acs
}

// ==================== Ignore rules ====================

const OS_GENERATED_FILES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini", "ehthumbs.db"];

struct IgnoreSet {
    patterns: Vec<String>,
}

/// Parse .gitignore patterns, merge with OS-generated file list
fn build_ignore_set(doc_root: &Path) -> IgnoreSet {
    let mut patterns = Vec::new();

    // Walk up to find .gitignore (ancestor of doc_root)
    let mut search = doc_root.to_path_buf();
    loop {
        let gitignore = search.join(".gitignore");
        if gitignore.is_file() {
            if let Ok(content) = fs::read_to_string(&gitignore) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    patterns.push(trimmed.to_string());
                }
            }
            break;
        }
        if !search.pop() {
            break;
        }
    }

    IgnoreSet { patterns }
}

/// Check if filename should be ignored
fn should_ignore(name: &str, ignore_set: &IgnoreSet) -> bool {
    // Skip OS-generated files
    if OS_GENERATED_FILES.contains(&name) {
        return true;
    }

    // Match .gitignore patterns (simplified: exact match or suffix wildcard)
    for pattern in &ignore_set.patterns {
        let pat = pattern.trim_end_matches('/');
        if pat == name {
            return true;
        }
        // *.ext wildcard
        if let Some(ext) = pat.strip_prefix("*.") {
            if name.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Regression: a frontmatter block with an opening `---` but no closing
    /// `---` must be reported as InvalidFrontmatter (not misreported as a
    /// missing required field).
    #[test]
    fn test_unclosed_frontmatter_reported_as_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("issue_test_2026-01-01_1.md");
        // Opening delimiter present, but no closing `---` anywhere.
        fs::write(
            &path,
            "---\nsource: test\nnums: 1\nthis never closes\n- [ ] ISSUE-I001: x\n",
        )
        .unwrap();

        let errors = validate_issue_file(&path);
        assert!(
            errors.iter().any(|e| e.kind == ErrorKind::InvalidFrontmatter),
            "expected InvalidFrontmatter for unclosed frontmatter, got: {:?}",
            errors.iter().map(|e| &e.kind).collect::<Vec<_>>()
        );
    }

    /// Sanity: a well-formed frontmatter (opening + closing `---`) must NOT
    /// be flagged InvalidFrontmatter.
    #[test]
    fn test_well_formed_frontmatter_not_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("issue_test_2026-01-01_2.md");
        fs::write(
            &path,
            "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I002: y\n",
        )
        .unwrap();

        let errors = validate_issue_file(&path);
        assert!(
            !errors.iter().any(|e| e.kind == ErrorKind::InvalidFrontmatter),
            "well-formed frontmatter must not be InvalidFrontmatter, got: {:?}",
            errors.iter().map(|e| &e.kind).collect::<Vec<_>>()
        );
    }
}
