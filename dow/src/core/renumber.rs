// dow/src/core/
// ├── renumber.rs  -- Shared task/issue ID renumber utility
//
// Single function: renumber all IDs sequentially by file date order.
// Used by both doctor --fix and rollback to resolve conflicts/gaps.
//
// Related Docs:
// - [Doctor](../commands/doctor.rs)
// - [Rollback](../commands/rollback.rs)

use crate::core::item_id::{self, ItemKind};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Result of a renumber operation
pub struct RenumberResult {
    /// Number of IDs that were changed
    pub changed: u32,
    /// Mapping of old_num → new_num for reporting
    pub renames: Vec<(u32, u32)>,
}

/// Renumber all IDs sequentially (1, 2, 3...) ordered by file date, file seq, line index.
/// Resolves both duplicate IDs and gaps. Modifies all matching files (pending + done/closed).
/// No-op if IDs are already sequential and conflict-free.
pub fn renumber(dir: &Path, kind: ItemKind) -> Result<RenumberResult, String> {
    if !dir.is_dir() {
        return Ok(RenumberResult { changed: 0, renames: vec![] });
    }

    let (done_prefix, pending_prefix, id_prefix) = match kind {
        ItemKind::Task => ("done_task_", "task_", "TASK-T"),
        ItemKind::Issue => ("closed_issue_", "issue_", "ISSUE-I"),
    };

    struct ItemEntry {
        file_path: std::path::PathBuf,
        file_date: String,
        file_seq: u32,
        line_idx: usize,
        current_num: u32,
    }

    let mut items: Vec<ItemEntry> = Vec::new();
    let mut seen_nums: HashSet<u32> = HashSet::new();
    let mut has_conflict = false;

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return Ok(RenumberResult { changed: 0, renames: vec![] }),
    };

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        if !name.starts_with(done_prefix) && !name.starts_with(pending_prefix) {
            continue;
        }

        let (date, seq) = extract_file_date_seq(&name);

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_idx, line) in content.lines().enumerate() {
            if !line.starts_with("- [") {
                continue;
            }
            if let Some(parsed) = item_id::extract_from_line(line) {
                if parsed.kind == kind {
                    let num = parsed.num();
                    if !seen_nums.insert(num) {
                        has_conflict = true;
                    }
                    items.push(ItemEntry {
                        file_path: entry.path(),
                        file_date: date.clone(),
                        file_seq: seq,
                        line_idx,
                        current_num: num,
                    });
                }
            }
        }
    }

    // Skip if already sequential and conflict-free
    if !has_conflict {
        if !items.is_empty() {
            let mut nums: Vec<u32> = items.iter().map(|i| i.current_num).collect();
            nums.sort();
            let is_sequential = nums
                .iter()
                .enumerate()
                .all(|(idx, &n)| n == (idx as u32 + 1));
            if is_sequential {
                return Ok(RenumberResult { changed: 0, renames: vec![] });
            }
        } else {
            return Ok(RenumberResult { changed: 0, renames: vec![] });
        }
    }

    // Sort by file date, file seq, line index (determines canonical order)
    items.sort_by(|a, b| {
        a.file_date
            .cmp(&b.file_date)
            .then(a.file_seq.cmp(&b.file_seq))
            .then(a.line_idx.cmp(&b.line_idx))
    });

    // Assign new sequential numbers
    let mut file_renames: HashMap<std::path::PathBuf, Vec<(usize, u32, u32)>> = HashMap::new();
    let mut global_renames: Vec<(u32, u32)> = Vec::new();

    for (new_idx, item) in items.iter().enumerate() {
        let new_num = (new_idx + 1) as u32;
        if new_num != item.current_num {
            file_renames.entry(item.file_path.clone()).or_default().push((
                item.line_idx,
                item.current_num,
                new_num,
            ));
            global_renames.push((item.current_num, new_num));
        }
    }

    if file_renames.is_empty() {
        return Ok(RenumberResult { changed: 0, renames: vec![] });
    }

    // Apply line-level replacements per file
    let mut total_changed = 0u32;
    for (file_path, changes) in &file_renames {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        for &(line_idx, old_num, new_num) in changes {
            if line_idx < lines.len() {
                let old_id = format!("{}{:03}", id_prefix, old_num);
                let new_id = format!("{}{:03}", id_prefix, new_num);
                lines[line_idx] = lines[line_idx].replace(&old_id, &new_id);
            }
        }
        let new_content = lines.join("\n");
        let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };
        fs::write(file_path, &final_content)
            .map_err(|e| format!("write {}: {}", file_path.display(), e))?;
        total_changed += changes.len() as u32;
    }

    global_renames.sort();
    global_renames.dedup();

    Ok(RenumberResult { changed: total_changed, renames: global_renames })
}

/// Extract (date, seq) from a task/issue filename.
fn extract_file_date_seq(name: &str) -> (String, u32) {
    let stem = name.trim_end_matches(".md");
    let date = find_date_in_name(stem).unwrap_or_default();
    let seq = stem
        .rsplit('_')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    (date, seq)
}

fn find_date_in_name(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    for i in 0..name.len().saturating_sub(9) {
        if bytes[i + 4] == b'-' && bytes[i + 7] == b'-' {
            let candidate = &name[i..i + 10];
            if candidate.len() == 10
                && candidate[..4].chars().all(|c| c.is_ascii_digit())
                && candidate[5..7].chars().all(|c| c.is_ascii_digit())
                && candidate[8..10].chars().all(|c| c.is_ascii_digit())
            {
                return Some(candidate.to_string());
            }
        }
    }
    None
}
