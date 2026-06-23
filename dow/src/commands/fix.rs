// dow/src/commands/
// ├── fix.rs  -- dow fix (auto-fix .dev-doc file format issues)
//
// Related Docs:
// - [ISSUE Specification](../../../references/.dev-doc/ISSUE.md)
// - [TASK Specification](../../../references/.dev-doc/TASK-FILE.md)

use crate::core::{doc_root, doc_validator};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct FixOutput {
    fixed: Vec<String>,
    unfixable: Vec<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();

    // Fix issue files
    let issue_dir = doc_root_path.join("issue");
    if issue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("issue_") || name.starts_with("closed_issue_"))
                {
                    let results = fix_issue_file(&entry.path());
                    fixed.extend(results.0);
                    unfixable.extend(results.1);
                }
            }
        }
    }

    // Fix task files
    let task_dir = doc_root_path.join("task");
    if task_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md")
                    && (name.starts_with("task_") || name.starts_with("done_task_"))
                {
                    let results = fix_task_file(&entry.path());
                    fixed.extend(results.0);
                    unfixable.extend(results.1);
                }
            }
        }
    }

    // Fix issue state consistency: add closed_ prefix to fully checked issue files
    fix_issue_rename(&issue_dir, &mut fixed);

    // Fix task state consistency: add done_ prefix to fully checked task files
    fix_task_rename(&task_dir, &mut fixed);

    // Fix issue global sequence conflicts: renumber by file date
    fix_issue_renumber(&issue_dir, &mut fixed);

    let result = FixOutput { fixed, unfixable };

    if human {
        print_human(&result);
    } else {
        output::print_json(&result);
    }

    if result.unfixable.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// Fix a single issue file, returns (fixed, unfixable) lists
fn fix_issue_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_issue_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                // Extract source from filename and add frontmatter
                let source = extract_source_from_filename(&filename).unwrap_or("other");
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\nsource: {}\nnums: {}\n---\n\n", source, item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}: added frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("source") {
                    let source = extract_source_from_filename(&filename).unwrap_or("other");
                    new_content = insert_fm_field(&new_content, "source", source);
                    needs_write = true;
                    fixed.push(format!("{}: added source field", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}: added nums field", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}: {}", filename, error.message));
            }
        }
    }

    if needs_write {
        if let Err(e) = fs::write(path, &new_content) {
            eprintln!("[dev-flow] Warning: failed to write {}: {}", filename, e);
            unfixable.push(format!("{}: write failed - {}", filename, e));
        }
    }

    (fixed, unfixable)
}

/// Fix a single task file, returns (fixed, unfixable) lists
fn fix_task_file(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut fixed = Vec::new();
    let mut unfixable = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (fixed, unfixable),
    };

    let errors = doc_validator::validate_task_file(path);
    if errors.is_empty() {
        return (fixed, unfixable);
    }

    let mut new_content = content.clone();
    let mut needs_write = false;

    for error in &errors {
        match error.kind {
            doc_validator::ErrorKind::MissingFrontmatter => {
                let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                let fm = format!("---\ntitle: TASK - \nnums: {}\n---\n\n", item_count);
                new_content = format!("{}{}", fm, new_content);
                needs_write = true;
                fixed.push(format!("{}: added frontmatter", filename));
            }
            doc_validator::ErrorKind::MissingRequiredField if error.fixable => {
                if error.message.contains("title") {
                    new_content = insert_fm_field(&new_content, "title", "TASK - ");
                    needs_write = true;
                    fixed.push(format!("{}: added title field", filename));
                } else if error.message.contains("nums") {
                    let item_count = new_content.lines().filter(|l| l.starts_with("- [")).count();
                    new_content = insert_fm_field(&new_content, "nums", &item_count.to_string());
                    needs_write = true;
                    fixed.push(format!("{}: added nums field", filename));
                }
            }
            _ => {
                unfixable.push(format!("{}: {}", filename, error.message));
            }
        }
    }

    if needs_write {
        if let Err(e) = fs::write(path, &new_content) {
            eprintln!("[dev-flow] Warning: failed to write {}: {}", filename, e);
            unfixable.push(format!("{}: write failed - {}", filename, e));
        }
    }

    (fixed, unfixable)
}

/// Extract source from issue filename
fn extract_source_from_filename(filename: &str) -> Option<&str> {
    let stem = filename.strip_suffix(".md")?;
    let rest = if stem.starts_with("closed_issue_") {
        &stem["closed_issue_".len()..]
    } else if stem.starts_with("issue_") {
        &stem["issue_".len()..]
    } else {
        return None;
    };
    // source_YYYY-MM-DD_seq
    rest.split('_').next()
}

/// Insert a field into existing frontmatter (if frontmatter exists but field is missing)
fn insert_fm_field(content: &str, key: &str, value: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    if let Some(end_idx) = content[3..].find("---") {
        let fm_end = 3 + end_idx;
        let mut fm = content[3..fm_end].to_string();
        fm.push_str(&format!("{}: {}\n", key, value));
        format!("---{}---{}", fm, &content[fm_end + 3..])
    } else {
        content.to_string()
    }
}

fn print_human(result: &FixOutput) {
    println!("[dev-flow] Document Format Fixes");
    println!("━━━━━━━━━━━━━━━━━━━━━━");

    if result.fixed.is_empty() && result.unfixable.is_empty() {
        println!("All files are correctly formatted, no fixes needed.");
        return;
    }

    if !result.fixed.is_empty() {
        println!("Fixed ({} items):", result.fixed.len());
        for item in &result.fixed {
            println!("  ✓ {}", item);
        }
        println!();
    }

    if !result.unfixable.is_empty() {
        println!("Requires manual fix ({} items):", result.unfixable.len());
        for item in &result.unfixable {
            println!("  ✗ {}", item);
        }
        println!();
        println!("Hint: The above issues cannot be auto-fixed, please edit the corresponding files manually.");
    }
}

/// Rename issue files with all items checked to closed_ prefix
fn fix_issue_rename(issue_dir: &Path, fixed: &mut Vec<String>) {
    if !issue_dir.is_dir() {
        return;
    }
    let entries: Vec<_> = match fs::read_dir(issue_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("issue_") || !name.ends_with(".md") {
            continue;
        }
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let total = content.lines().filter(|l| l.starts_with("- [")).count();
        let done = content.lines().filter(|l| l.starts_with("- [x]")).count();
        if total > 0 && total == done {
            let new_name = format!("closed_{}", name);
            let new_path = issue_dir.join(&new_name);
            if !new_path.exists() {
                if let Err(e) = fs::rename(entry.path(), &new_path) {
                    eprintln!("[dow fix] Warning: failed to rename {} → {}: {}", name, new_name, e);
                } else {
                    fixed.push(format!("{}: renamed to {}", name, new_name));
                }
            }
        }
    }
}

/// Rename task files with all items checked to done_ prefix
fn fix_task_rename(task_dir: &Path, fixed: &mut Vec<String>) {
    if !task_dir.is_dir() {
        return;
    }
    let entries: Vec<_> = match fs::read_dir(task_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("task_") || !name.ends_with(".md") {
            continue;
        }
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let total = content.lines().filter(|l| l.starts_with("- [")).count();
        let done = content.lines().filter(|l| l.starts_with("- [x]")).count();
        if total > 0 && total == done {
            let new_name = format!("done_{}", name);
            let new_path = task_dir.join(&new_name);
            if !new_path.exists() {
                if let Err(e) = fs::rename(entry.path(), &new_path) {
                    eprintln!("[dow fix] Warning: failed to rename {} → {}: {}", name, new_name, e);
                } else {
                    fixed.push(format!("{}: renamed to {}", name, new_name));
                }
            }
        }
    }
}

/// Fix issue global sequence conflicts: reassign continuous numbers sorted by file date + sequence
fn fix_issue_renumber(issue_dir: &Path, fixed: &mut Vec<String>) {
    if !issue_dir.is_dir() {
        return;
    }

    struct IssueItem {
        file_path: std::path::PathBuf,
        file_date: String,
        file_seq: u32,
        line_idx: usize,
        current_num: u32,
    }

    let mut items: Vec<IssueItem> = Vec::new();
    let mut seen_nums: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut has_conflict = false;

    let entries: Vec<_> = match fs::read_dir(issue_dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        if !name.starts_with("issue_") && !name.starts_with("closed_issue_") {
            continue;
        }

        let (date, seq) = match parse_issue_file_date_seq(&name) {
            Some(v) => v,
            None => continue,
        };

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_idx, line) in content.lines().enumerate() {
            if !line.starts_with("- [") {
                continue;
            }
            let title = line[5..].trim();
            if let Some(num) = extract_issue_num(title) {
                if !seen_nums.insert(num) {
                    has_conflict = true;
                }
                items.push(IssueItem {
                    file_path: entry.path(),
                    file_date: date.clone(),
                    file_seq: seq,
                    line_idx,
                    current_num: num,
                });
            }
        }
    }

    if !has_conflict {
        if !items.is_empty() {
            let mut nums: Vec<u32> = items.iter().map(|i| i.current_num).collect();
            nums.sort();
            let is_sequential = nums.iter().enumerate().all(|(idx, &n)| n == (idx as u32 + 1));
            if is_sequential {
                return;
            }
        } else {
            return;
        }
    }

    // Sort by file date, file sequence, and line index
    items.sort_by(|a, b| {
        a.file_date.cmp(&b.file_date)
            .then(a.file_seq.cmp(&b.file_seq))
            .then(a.line_idx.cmp(&b.line_idx))
    });

    // Assign new sequence numbers
    let mut renames: std::collections::HashMap<std::path::PathBuf, Vec<(usize, u32, u32)>> =
        std::collections::HashMap::new();
    for (new_idx, item) in items.iter().enumerate() {
        let new_num = (new_idx + 1) as u32;
        if new_num != item.current_num {
            renames
                .entry(item.file_path.clone())
                .or_default()
                .push((item.line_idx, item.current_num, new_num));
        }
    }

    if renames.is_empty() {
        return;
    }

    // Apply replacements
    let mut total_fixed = 0u32;
    for (file_path, changes) in &renames {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        for &(line_idx, old_num, new_num) in changes {
            if line_idx < lines.len() {
                let old_id = format!("ISSUE-I{:03}", old_num);
                let new_id = format!("ISSUE-I{:03}", new_num);
                lines[line_idx] = lines[line_idx].replace(&old_id, &new_id);
            }
        }
        let new_content = lines.join("\n");
        let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };
        if let Err(e) = fs::write(file_path, &final_content) {
            eprintln!("[dow fix] Warning: failed to write {}: {}", file_path.display(), e);
        } else {
            total_fixed += changes.len() as u32;
        }
    }

    if total_fixed > 0 {
        fixed.push(format!("issue global sequence renumbering: fixed {} items", total_fixed));
    }
}

fn parse_issue_file_date_seq(filename: &str) -> Option<(String, u32)> {
    let stem = filename.strip_suffix(".md")?;
    let rest = if stem.starts_with("closed_issue_") {
        &stem["closed_issue_".len()..]
    } else if stem.starts_with("issue_") {
        &stem["issue_".len()..]
    } else {
        return None;
    };
    // rest = "source_YYYY-MM-DD_seq"
    let parts: Vec<&str> = rest.splitn(3, '_').collect();
    if parts.len() < 3 {
        return None;
    }
    let date = parts[1].to_string();
    let seq = parts[2].parse::<u32>().unwrap_or(0);
    Some((date, seq))
}

fn extract_issue_num(title: &str) -> Option<u32> {
    let prefix = "ISSUE-I";
    if !title.starts_with(prefix) {
        return None;
    }
    let rest = &title[prefix.len()..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}
