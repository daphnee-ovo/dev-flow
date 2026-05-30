// dow/src/commands/
// ├── validate.rs  -- dow validate（校验 .dev-doc 目录结构与文件规范）

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
    // 0. 旧版 dev-doc/ 目录迁移检测
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

    // 1. 目录结构校验（自动创建缺失目录）
    check_directories(&doc_root_path, &mut result);

    // 2. 统一文档校验
    collect_validation_errors(&doc_root_path, &mut result);

    // 3. CHANGELOG 校验
    check_changelog(&doc_root_path, &mut result);

    // 4. .gitignore 检查
    check_gitignore(&mut result);

    // 5. 根级残留文件检查
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

    // 按 (kind, message) 归类，合并同类文件
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

    if msg.contains("非工作流文件") {
        (
            "illegal_files".into(),
            Some("move to docs/ or remove (valid: PRD/SPEC/TEST/BRAINSTORM/CHANGELOG/STATUS.yaml)".into()),
        )
    } else if msg.contains("非工作流目录") {
        (
            "illegal_dirs".into(),
            Some("remove if migrated to SQLite (valid: task/, issue/)".into()),
        )
    } else if msg.contains("只允许 task_") || msg.contains("只允许 issue_") {
        (
            "illegal_subdir_files".into(),
            Some("remove or rename to valid pattern".into()),
        )
    } else if msg.contains("未重命名为 closed_") || msg.contains("已勾选但文件未重命名") {
        (
            "issue_status_mismatch".into(),
            Some("rename to closed_ prefix".into()),
        )
    } else if msg.contains("closed_ 前缀但存在未勾选") {
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
    } else if msg.contains("缺少必填字段") || msg.contains("缺少 ") {
        ("missing_required_field".into(), None)
    } else if msg.contains("序号") {
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
            fs::create_dir_all(dir).ok();
            result.auto_fixed.push(format!("created_dir:{}", dir.display()));
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
        fs::write(&changelog, "# Changelog\n").ok();
        result.auto_fixed.push("created_changelog".to_string());
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
            fs::write(".gitignore", new_content).ok();
            result.auto_fixed.push("gitignore_added_project_temp".to_string());
        }
    } else {
        fs::write(".gitignore", format!("{}\n", project_temp)).ok();
        result.auto_fixed.push("gitignore_created".to_string());
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

/// 检测旧版 dev-doc/ 目录是否存在且含 dev-flow 特征文件
/// 只有包含 STATUS.yaml 的才视为 dev-flow 管理的旧目录需要迁移
fn check_legacy_doc_dir() -> Option<String> {
    let legacy = Path::new(crate::core::DOC_DIR_LEGACY);
    if !legacy.is_dir() {
        return None;
    }
    // 检查是否含 dev-flow 特征：任意子目录中有 STATUS.yaml
    let has_status = fs::read_dir(legacy).ok()?.flatten().any(|e| {
        e.path().is_dir() && e.path().join("STATUS.yaml").exists()
    });
    // 顶层也可能有（旧格式）
    let has_top_status = legacy.join("STATUS.yaml").exists();
    if !has_status && !has_top_status {
        return None;
    }
    Some(format!(
        "[dev-flow] 检测到旧版文档目录 `dev-doc/`（含 STATUS.yaml）。\n\
         dev-flow 已迁移到 `.dev-doc/`，请执行 `mv dev-doc .dev-doc` 完成迁移，否则 dev-flow 无法正常工作。"
    ))
}
