// dow/src/commands/
// ├── validate.rs  -- dow validate (validate .dev-doc directory structure and file format)

use crate::core::{doc_root, doc_validator};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
struct ValidateItem {
    r#type: String,
    message: String,
    files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

#[derive(Serialize)]
struct ValidateOutput {
    doc_root: String,
    auto_fixed: Vec<String>,
    needs_confirm: Vec<ValidateItem>,
    warnings: Vec<ValidateItem>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    // 0. Legacy dev-doc/ directory migration detection
    if let Some(msg) = check_legacy_doc_dir() {
        if human {
            println!("{}", msg);
        } else {
            let warn = serde_json::json!({"legacy_migration": msg});
            println!("{}", warn);
        }
        return Ok(2);
    }

    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let mut result = ValidateOutput {
        doc_root: doc_root_path.to_string_lossy().to_string(),
        auto_fixed: Vec::new(),
        needs_confirm: Vec::new(),
        warnings: Vec::new(),
    };

    // 1. Directory structure validation (auto-create missing directories)
    check_directories(&doc_root_path, &mut result);

    // 2. Unified document validation
    collect_validation_errors(&doc_root_path, &mut result);

    // 3. CHANGELOG validation
    check_changelog(&doc_root_path, &mut result);

    // 4. .gitignore check
    check_gitignore(&mut result);

    // 5. Root-level stale file check
    check_stale_root_files(&doc_root_path, &mut result);

    let has_problems = !result.needs_confirm.is_empty() || !result.warnings.is_empty();

    if human {
        print_human(&result);
    } else {
        output::print_json(&result);
    }

    Ok(if has_problems { 1 } else { 0 })
}

fn collect_validation_errors(doc_root: &Path, result: &mut ValidateOutput) {
    let errors = doc_validator::validate_all(doc_root);
    if errors.is_empty() {
        return;
    }

    // Group by (kind, message), merge files of the same type
    use std::collections::BTreeMap;

    struct Group {
        message: String,
        files: Vec<String>,
        fixable: bool,
        hint: Option<String>,
    }

    let mut groups: BTreeMap<String, Group> = BTreeMap::new();

    for e in errors {
        let (type_key, hint) = classify_error(&e);

        groups
            .entry(type_key.clone())
            .and_modify(|g| {
                if !g.files.contains(&e.file) {
                    g.files.push(e.file.clone());
                }
            })
            .or_insert_with(|| Group {
                message: e.message.clone(),
                files: vec![e.file.clone()],
                fixable: e.fixable,
                hint,
            });
    }

    for (type_key, group) in groups {
        let item = ValidateItem {
            r#type: type_key,
            message: group.message,
            files: group.files,
            hint: group.hint,
        };
        if group.fixable {
            result.needs_confirm.push(item);
        } else {
            result.warnings.push(item);
        }
    }
}

fn classify_error(e: &doc_validator::ValidationError) -> (String, Option<String>) {
    let msg = &e.message;

    if msg.contains("non-workflow file") {
        (
            "illegal_files".into(),
            Some("move to docs/ or remove (valid: PRD/SPEC/TEST/BRAINSTORM/CHANGELOG/STATUS.yaml)".into()),
        )
    } else if msg.contains("non-workflow directory") {
        (
            "illegal_dirs".into(),
            Some("remove if migrated to SQLite (valid: task/, issue/)".into()),
        )
    } else if msg.contains("only task_") || msg.contains("only issue_") {
        (
            "illegal_subdir_files".into(),
            Some("remove or rename to valid pattern".into()),
        )
    } else if msg.contains("not renamed to closed_") || msg.contains("checked but file not renamed") {
        (
            "issue_status_mismatch".into(),
            Some("rename to closed_ prefix".into()),
        )
    } else if msg.contains("closed_ prefix but unchecked items exist") {
        (
            "issue_closed_but_open".into(),
            Some("reopen items or remove closed_ prefix".into()),
        )
    } else if msg.contains("STATUS.yaml") {
        ("status_yaml".into(), None)
    } else if msg.contains("SPEC-AC") {
        ("spec_ac_sequence".into(), None)
    } else if msg.contains("frontmatter") {
        ("missing_frontmatter".into(), Some("run `dow fix` to auto-fix".into()))
    } else if msg.contains("priority") || msg.contains("severity") || msg.contains("complexity") {
        ("invalid_field_value".into(), None)
    } else if msg.contains("missing required field") || msg.contains("missing ") {
        ("missing_required_field".into(), None)
    } else if msg.contains("sequence") {
        ("sequence_error".into(), None)
    } else {
        ("other".into(), None)
    }
}

fn check_directories(doc_root: &Path, result: &mut ValidateOutput) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp"
    } else {
        "tmp"
    };

    let dirs = [
        doc_root.join("issue"),
        doc_root.join("task"),
        PathBuf::from("tests"),
        PathBuf::from(project_temp),
    ];

    for dir in &dirs {
        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("[dow] warning: failed to create directory ({}): {}", dir.display(), e);
            } else {
                result.auto_fixed.push(format!("created_dir:{}", dir.display()));
            }
        }
    }
}

fn check_changelog(doc_root: &Path, result: &mut ValidateOutput) {
    let changelog = doc_root.join("CHANGELOG.md");
    if changelog.exists() {
        if fs::metadata(&changelog).map(|m| m.len() == 0).unwrap_or(true) {
            result.warnings.push(ValidateItem {
                r#type: "changelog_empty".into(),
                message: "CHANGELOG.md is empty".into(),
                files: vec!["CHANGELOG.md".into()],
                hint: Some("add entries or remove file".into()),
            });
        }
    } else {
        if let Err(e) = fs::write(&changelog, "# Changelog\n") {
            eprintln!("[dow] warning: failed to create CHANGELOG.md: {}", e);
        } else {
            result.auto_fixed.push("created_changelog".to_string());
        }
    }
}

fn check_stale_root_files(doc_root: &Path, result: &mut ValidateOutput) {
    let base_path = Path::new(crate::core::DOC_DIR);
    if doc_root == base_path {
        return;
    }

    let mut stale_files = Vec::new();
    for subdir in &["issue", "task"] {
        let root_dir = base_path.join(subdir);
        if !root_dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&root_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                stale_files.push(format!("{}/{}", subdir, name));
            }
        }
    }

    if !stale_files.is_empty() {
        result.needs_confirm.push(ValidateItem {
            r#type: "stale_root_files".into(),
            message: "files in .dev-doc/ root should be in branch subdirectory".into(),
            files: stale_files,
            hint: Some(format!("move to {}/", doc_root.display())),
        });
    }
}

fn check_gitignore(result: &mut ValidateOutput) {
    let project_temp = if Path::new("temp").is_dir() && !Path::new("tmp").is_dir() {
        "temp/"
    } else {
        "tmp/"
    };

    if Path::new(".gitignore").exists() {
        let content = fs::read_to_string(".gitignore").unwrap_or_default();
        if !content.lines().any(|l| l.trim() == project_temp) {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(project_temp);
            new_content.push('\n');
            if let Err(e) = fs::write(".gitignore", new_content) {
                eprintln!("[dow] warning: failed to update .gitignore: {}", e);
            } else {
                result.auto_fixed.push("gitignore_added_project_temp".to_string());
            }
        }
    } else {
        if let Err(e) = fs::write(".gitignore", format!("{}\n", project_temp)) {
            eprintln!("[dow] warning: failed to create .gitignore: {}", e);
        } else {
            result.auto_fixed.push("gitignore_created".to_string());
        }
    }
}

fn print_human(result: &ValidateOutput) {
    println!("[dev-flow] .dev-doc validation report");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("doc_root: {}", result.doc_root);
    println!();

    if !result.auto_fixed.is_empty() {
        println!("auto_fixed ({}):", result.auto_fixed.len());
        for item in &result.auto_fixed {
            println!("  - {}", item);
        }
        println!();
    }

    if !result.needs_confirm.is_empty() {
        println!("needs_confirm ({}):", result.needs_confirm.len());
        for (i, item) in result.needs_confirm.iter().enumerate() {
            println!("{}. [{}] {}", i + 1, item.r#type, item.message);
            println!("   - files: [{}]", item.files.join(", "));
            if let Some(ref hint) = item.hint {
                println!("   - hint: {}", hint);
            }
        }
        println!();
    }

    if !result.warnings.is_empty() {
        println!("warnings ({}):", result.warnings.len());
        for (i, item) in result.warnings.iter().enumerate() {
            println!("{}. [{}] {}", i + 1, item.r#type, item.message);
            println!("   - files: [{}]", item.files.join(", "));
            if let Some(ref hint) = item.hint {
                println!("   - hint: {}", hint);
            }
        }
        println!();
    }

    if result.needs_confirm.is_empty() && result.warnings.is_empty() {
        println!("all passed.");
    }
}

/// Detect if legacy dev-doc/ directory exists and contains dev-flow characteristic files
/// Only those containing STATUS.yaml are considered old directories managed by dev-flow that need migration
fn check_legacy_doc_dir() -> Option<String> {
    let legacy = Path::new(crate::core::DOC_DIR_LEGACY);
    if !legacy.is_dir() {
        return None;
    }
    // Check if it contains dev-flow characteristics: STATUS.yaml in any subdirectory
    let has_status = fs::read_dir(legacy).ok()?.flatten().any(|e| {
        e.path().is_dir() && e.path().join("STATUS.yaml").exists()
    });
    // Top level may also have it (old format)
    let has_top_status = legacy.join("STATUS.yaml").exists();
    if !has_status && !has_top_status {
        return None;
    }
    Some(format!(
        "[dev-flow] Detected legacy documentation directory `dev-doc/` (containing STATUS.yaml).\n\
         dev-flow has migrated to `.dev-doc/`, please execute `mv dev-doc .dev-doc` to complete migration, otherwise dev-flow will not work properly."
    ))
}
