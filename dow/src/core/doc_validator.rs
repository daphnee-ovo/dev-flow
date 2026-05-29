// dow/src/core/
// ├── doc_validator.rs  -- dev-doc 文件合法性校验
//    从 references/dev-doc/*.md 编译时嵌入规范，运行时解析并校验文件
//
// Related Docs:
// - [ISSUE 规范](../../../references/dev-doc/ISSUE.md)
// - [TASK 规范](../../../references/dev-doc/TASK-FILE.md)

use std::fs;
use std::path::Path;

// 编译时嵌入规范 md
const REF_ISSUE: &str = include_str!("../../../references/dev-doc/ISSUE.md");
const REF_TASK: &str = include_str!("../../../references/dev-doc/TASK-FILE.md");

/// 验证错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub file: String,
    pub kind: ErrorKind,
    pub message: String,
    /// 是否可通过 dow fix 自动修复
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

/// 从 ISSUE.md 规范解析出的验证规则
struct IssueSpec {
    valid_sources: Vec<String>,
    valid_severities: Vec<String>,
}

/// 从 TASK-FILE.md 规范解析出的验证规则
struct TaskSpec {
    valid_priorities: Vec<String>,
    valid_complexities: Vec<String>,
    required_fields: Vec<String>,
}

/// 解析 ISSUE.md 规范
fn parse_issue_spec() -> IssueSpec {
    let mut valid_sources = Vec::new();
    let mut valid_severities = Vec::new();

    for line in REF_ISSUE.lines() {
        // 从字段说明表格提取枚举值
        // 格式：| source | `test` / `devtest` / `other` / `audit` | ... |
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

/// 解析 TASK-FILE.md 规范
fn parse_task_spec() -> TaskSpec {
    let mut valid_priorities = Vec::new();
    let mut valid_complexities = Vec::new();
    let mut required_fields = Vec::new();

    let mut in_fields_table = false;

    for line in REF_TASK.lines() {
        // 检测字段说明表格区域
        if line.starts_with("## 字段说明") {
            in_fields_table = true;
            continue;
        }
        if in_fields_table && line.starts_with("## ") {
            in_fields_table = false;
        }

        // priority 枚举（从表格或 Priority 定义部分）
        if line.contains("| priority") && line.contains("P0") {
            valid_priorities = extract_enum_values(line);
        }
        // complexity 枚举
        if line.contains("| complexity") || (line.contains("| `S`") && line.contains("小任务")) {
            if valid_complexities.is_empty() {
                valid_complexities = extract_complexity_values(REF_TASK);
            }
        }
        // done_when 是必填（规范中说"必须客观具体"）
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

/// 从 md 表格行中提取 `value` / `value` 格式的枚举值
fn extract_enum_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    // 匹配 `xxx` 格式
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

/// 从 TASK-FILE.md 提取 complexity 枚举值（Complexity 定义表格）
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
            // | `S` | 小任务 | ...
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

// ==================== 验证逻辑 ====================

/// 验证单个 issue 文件
pub fn validate_issue_file(path: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let spec = parse_issue_spec();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // 1. 文件名验证
    if let Some(e) = validate_issue_filename(&filename, &spec) {
        errors.push(e);
    }

    // 2. 内容验证
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    errors.extend(validate_issue_content(&filename, &content, &spec));
    errors
}

/// 验证单个 task 文件
pub fn validate_task_file(path: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let spec = parse_task_spec();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // 1. 文件名验证
    if let Some(e) = validate_task_filename(&filename) {
        errors.push(e);
    }

    // 2. 内容验证
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    errors.extend(validate_task_content(&filename, &content, &spec));
    errors
}

/// 验证 issue 文件名
/// 合法格式：issue_<source>_<YYYY-MM-DD>_<seq>.md 或 closed_issue_<source>_<YYYY-MM-DD>_<seq>.md
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
            message: "文件名不符合规范（应为 issue_<source>_<YYYY-MM-DD>_<seq>.md）".into(),
            fixable: false,
        });
    };

    // 至少需要 source + 日期年 + 日期月日 + seq = 从 splitn(4, '_') 得到的 parts
    // 实际：source_YYYY-MM-DD_seq → splitn(4, '_') → [source, YYYY-MM-DD, seq] 不对
    // issue_test_2026-05-29_1.md → stem去掉prefix后 → "test_2026-05-29_1"
    // splitn(4, '_') → ["test", "2026-05-29", "1"]  — 不对，"-" 不是 split 字符
    // 实际 split('_') → ["test", "2026-05-29", "1"]
    if parts.len() < 3 {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "文件名缺少必要部分（需要 source、日期、序号）".into(),
            fixable: false,
        });
    }

    let source = parts[0];
    if !spec.valid_sources.iter().any(|s| s == source) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!(
                "source '{}' 不合法，有效值：{}",
                source,
                spec.valid_sources.join("/")
            ),
            fixable: false,
        });
    }

    // 验证日期格式 YYYY-MM-DD
    let date = parts[1];
    if !is_valid_date(date) {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!("日期 '{}' 格式不合法（需要 YYYY-MM-DD）", date),
            fixable: false,
        });
    }

    // 验证序号为数字
    let seq = parts[2];
    if seq.parse::<u32>().is_err() {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: format!("序号 '{}' 不是有效数字", seq),
            fixable: false,
        });
    }

    None
}

/// 验证 task 文件名
/// 合法格式：task_<YYYY-MM-DD>_<seq>.md 或 done_task_<YYYY-MM-DD>_<seq>.md
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
            message: "文件名不符合规范（应为 task_<YYYY-MM-DD>_<seq>.md）".into(),
            fixable: false,
        });
    };

    // name_part 应为 "YYYY-MM-DD_seq"，按最后一个 _ 分割（日期里没有 _）
    if let Some(last_underscore) = name_part.rfind('_') {
        let date = &name_part[..last_underscore];
        let seq = &name_part[last_underscore + 1..];

        if !is_valid_date(date) {
            return Some(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::BadFilename,
                message: format!("日期 '{}' 格式不合法（需要 YYYY-MM-DD）", date),
                fixable: false,
            });
        }
        if seq.parse::<u32>().is_err() {
            return Some(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::BadFilename,
                message: format!("序号 '{}' 不是有效数字", seq),
                fixable: false,
            });
        }
    } else {
        return Some(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::BadFilename,
            message: "文件名缺少序号部分".into(),
            fixable: false,
        });
    }

    None
}

/// 验证 issue 文件内容
fn validate_issue_content(filename: &str, content: &str, spec: &IssueSpec) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 检查 YAML frontmatter
    if !content.starts_with("---") {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingFrontmatter,
            message: "缺少 YAML frontmatter（---）".into(),
            fixable: true,
        });
    } else {
        let fm = extract_frontmatter(content);
        // source 字段
        if let Some(source_val) = extract_fm_value(&fm, "source") {
            if !spec.valid_sources.contains(&source_val) {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "frontmatter source '{}' 不合法，有效值：{}",
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
                message: "frontmatter 缺少 source 字段".into(),
                fixable: true,
            });
        }
        // nums 字段
        if extract_fm_value(&fm, "nums").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter 缺少 nums 字段".into(),
                fixable: true,
            });
        }
    }

    // 检查每个 issue item 的 severity
    let mut in_item = false;
    let mut item_title = String::new();
    let mut has_severity = false;

    for line in content.lines() {
        if line.starts_with("- [ ]") || line.starts_with("- [x]") {
            // 之前 item 没有 severity
            if in_item && !has_severity {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::MissingRequiredField,
                    message: format!("issue item '{}' 缺少 severity 字段", item_title),
                    fixable: false,
                });
            }
            in_item = true;
            item_title = line[5..].trim().to_string();
            has_severity = false;
        } else if in_item && line.contains("severity:") {
            has_severity = true;
            let val = line.split("severity:").nth(1).unwrap_or("").trim();
            if !spec.valid_severities.iter().any(|s| s == val) {
                errors.push(ValidationError {
                    file: filename.to_string(),
                    kind: ErrorKind::InvalidFieldValue,
                    message: format!(
                        "severity '{}' 不合法，有效值：{}",
                        val,
                        spec.valid_severities.join("/")
                    ),
                    fixable: false,
                });
            }
        }
    }
    // 最后一个 item
    if in_item && !has_severity {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingRequiredField,
            message: format!("issue item '{}' 缺少 severity 字段", item_title),
            fixable: false,
        });
    }

    errors
}

/// 验证 task 文件内容
fn validate_task_content(filename: &str, content: &str, spec: &TaskSpec) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 检查 YAML frontmatter
    if !content.starts_with("---") {
        errors.push(ValidationError {
            file: filename.to_string(),
            kind: ErrorKind::MissingFrontmatter,
            message: "缺少 YAML frontmatter（---）".into(),
            fixable: true,
        });
    } else {
        let fm = extract_frontmatter(content);
        if extract_fm_value(&fm, "title").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter 缺少 title 字段".into(),
                fixable: true,
            });
        }
        if extract_fm_value(&fm, "nums").is_none() {
            errors.push(ValidationError {
                file: filename.to_string(),
                kind: ErrorKind::MissingRequiredField,
                message: "frontmatter 缺少 nums 字段".into(),
                fixable: true,
            });
        }
    }

    // 检查每个 task item 的必填字段
    let mut in_item = false;
    let mut item_title = String::new();
    let mut found_fields: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.starts_with("- [ ]") || line.starts_with("- [x]") {
            // 检查上一个 item
            check_task_item_fields(filename, &item_title, &found_fields, spec, &mut errors);
            in_item = true;
            item_title = line[5..].trim().to_string();
            found_fields.clear();
        } else if in_item {
            // 收集字段
            let trimmed = line.trim_start();
            if trimmed.starts_with("- priority:") {
                found_fields.push("priority".into());
                let val = trimmed.split("priority:").nth(1).unwrap_or("").trim();
                if !spec.valid_priorities.iter().any(|p| p == val) {
                    errors.push(ValidationError {
                        file: filename.to_string(),
                        kind: ErrorKind::InvalidFieldValue,
                        message: format!(
                            "task '{}' priority '{}' 不合法，有效值：{}",
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
                            "task '{}' complexity '{}' 不合法，有效值：{}",
                            item_title, val, spec.valid_complexities.join("/")
                        ),
                        fixable: false,
                    });
                }
            } else if trimmed.starts_with("- done_when:") {
                found_fields.push("done_when".into());
            }
        }
    }
    // 最后一个 item
    check_task_item_fields(filename, &item_title, &found_fields, spec, &mut errors);

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
                message: format!("task '{}' 缺少必填字段 '{}'", item_title, req),
                fixable: false,
            });
        }
    }
}

// ==================== 批量验证入口 ====================

/// 验证指定目录下所有 issue 文件，返回错误列表
pub fn validate_all_issues(doc_root: &Path) -> Vec<ValidationError> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return vec![];
    }

    let mut all_errors = Vec::new();
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("issue_") || name.starts_with("closed_issue_"))
            {
                all_errors.extend(validate_issue_file(&entry.path()));
            }
        }
    }
    all_errors
}

/// 验证指定目录下所有 task 文件，返回错误列表
pub fn validate_all_tasks(doc_root: &Path) -> Vec<ValidationError> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return vec![];
    }

    let mut all_errors = Vec::new();
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("task_") || name.starts_with("done_task_"))
            {
                all_errors.extend(validate_task_file(&entry.path()));
            }
        }
    }
    all_errors
}

/// 验证所有 dev-doc 文件（task + issue），返回错误列表
pub fn validate_all(doc_root: &Path) -> Vec<ValidationError> {
    let mut errors = validate_all_issues(doc_root);
    errors.extend(validate_all_tasks(doc_root));
    errors
}

/// 格式化错误为人类可读输出
pub fn format_errors_human(errors: &[ValidationError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&format!("[dev-flow] 文档校验失败（{} 个错误）：\n", errors.len()));
    for e in errors {
        let fixable_hint = if e.fixable { " [可修复]" } else { "" };
        out.push_str(&format!("  - {}：{}{}\n", e.file, e.message, fixable_hint));
    }
    out.push_str("\n提示：运行 `dow fix` 自动修复可修复的问题。\n");
    out
}

// ==================== 工具函数 ====================

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
