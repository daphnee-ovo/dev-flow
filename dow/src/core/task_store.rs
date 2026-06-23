// dow/src/core/task_store.rs
// Task file traversal and status check utilities

use std::fs;
use std::path::{Path, PathBuf};

/// Traverse task directory, return all task_ prefixed .md file paths
pub fn iter_task_files(task_dir: &Path) -> Vec<PathBuf> {
    if !task_dir.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(task_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.starts_with("task_") && name.ends_with(".md")
        })
        .map(|e| e.path())
        .collect()
}

/// Check if file content has uncompleted checklist items
pub fn has_undone_items(content: &str) -> bool {
    content.lines().any(|l| l.starts_with("- [ ]"))
}

/// Count checklist item completion in file content
/// Returns (done_count, total_count)
pub fn count_checklist(content: &str) -> (usize, usize) {
    let done = content.lines().filter(|l| l.starts_with("- [x]")).count();
    let total = content.lines().filter(|l| l.starts_with("- [")).count();
    (done, total)
}

/// Check if task directory has any uncompleted checklist items
pub fn has_active_work(task_dir: &Path) -> bool {
    let task_files = iter_task_files(task_dir);

    for path in task_files {
        if let Ok(content) = fs::read_to_string(&path) {
            if has_undone_items(&content) {
                return true;
            }
        }
    }

    false
}

/// Count total uncompleted checklist items across all active task files
pub fn count_undone_items(task_dir: &Path) -> u32 {
    let task_files = iter_task_files(task_dir);
    let mut undone = 0u32;

    for path in task_files {
        if let Ok(content) = fs::read_to_string(&path) {
            undone += content.lines().filter(|l| l.starts_with("- [ ]")).count() as u32;
        }
    }

    undone
}

/// Count checklist completion across all active task files
/// Returns (done_total, item_total)
pub fn count_all_checklist(task_dir: &Path) -> (u32, u32) {
    let task_files = iter_task_files(task_dir);
    let mut done = 0u32;
    let mut total = 0u32;

    for path in task_files {
        if let Ok(content) = fs::read_to_string(&path) {
            let (d, t) = count_checklist(&content);
            done += d as u32;
            total += t as u32;
        }
    }

    (done, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn setup_tmp_dir(name: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tmp")
            .join(format!("test_task_store_{}_{}", name, id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_iter_task_files_filters_correctly() {
        let task_dir = setup_tmp_dir("iter_filter");
        fs::write(task_dir.join("task_2026-05-31_1.md"), "content").unwrap();
        fs::write(task_dir.join("done_task_2026-05-31_1.md"), "content").unwrap();
        fs::write(task_dir.join("notes.md"), "content").unwrap();
        fs::write(task_dir.join("task_other.txt"), "content").unwrap();

        let files = iter_task_files(&task_dir);
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap().to_str().unwrap().starts_with("task_"));
        assert!(files[0].file_name().unwrap().to_str().unwrap().ends_with(".md"));
    }

    #[test]
    fn test_iter_task_files_empty_dir() {
        let task_dir = setup_tmp_dir("iter_empty");
        let files = iter_task_files(&task_dir);
        assert!(files.is_empty());
    }

    #[test]
    fn test_iter_task_files_nonexistent_dir() {
        let files = iter_task_files(Path::new("/nonexistent_path_xyz"));
        assert!(files.is_empty());
    }

    #[test]
    fn test_has_undone_items_true() {
        let content = "- [x] TASK-T001: done\n- [ ] TASK-T002: pending\n";
        assert!(has_undone_items(content));
    }

    #[test]
    fn test_has_undone_items_false() {
        let content = "- [x] TASK-T001: done\n- [x] TASK-T002: also done\n";
        assert!(!has_undone_items(content));
    }

    #[test]
    fn test_has_undone_items_empty() {
        assert!(!has_undone_items(""));
        assert!(!has_undone_items("no checklist here\n"));
    }

    #[test]
    fn test_count_checklist() {
        let content = "- [x] done1\n- [ ] todo1\n- [x] done2\n- [ ] todo2\n- [ ] todo3\n";
        let (done, total) = count_checklist(content);
        assert_eq!(done, 2);
        assert_eq!(total, 5);
    }

    #[test]
    fn test_count_checklist_empty() {
        let (done, total) = count_checklist("");
        assert_eq!(done, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_has_active_work_with_undone() {
        let task_dir = setup_tmp_dir("active_undone");
        fs::write(
            task_dir.join("task_2026-05-31_1.md"),
            "- [x] done\n- [ ] pending\n",
        )
        .unwrap();

        assert!(has_active_work(&task_dir));
    }

    #[test]
    fn test_has_active_work_all_done() {
        let task_dir = setup_tmp_dir("active_done");
        fs::write(
            task_dir.join("task_2026-05-31_1.md"),
            "- [x] done\n- [x] also done\n",
        )
        .unwrap();

        assert!(!has_active_work(&task_dir));
    }

    #[test]
    fn test_has_active_work_empty_dir() {
        let task_dir = setup_tmp_dir("active_empty");
        assert!(!has_active_work(&task_dir));
    }

    #[test]
    fn test_count_undone_items() {
        let task_dir = setup_tmp_dir("count_undone");
        fs::write(
            task_dir.join("task_a.md"),
            "- [ ] todo1\n- [x] done1\n- [ ] todo2\n",
        )
        .unwrap();
        fs::write(task_dir.join("task_b.md"), "- [ ] todo3\n").unwrap();

        assert_eq!(count_undone_items(&task_dir), 3);
    }

    #[test]
    fn test_count_all_checklist() {
        let task_dir = setup_tmp_dir("count_all");
        fs::write(
            task_dir.join("task_a.md"),
            "- [x] done1\n- [ ] todo1\n",
        )
        .unwrap();
        fs::write(
            task_dir.join("task_b.md"),
            "- [x] done2\n- [x] done3\n- [ ] todo2\n",
        )
        .unwrap();

        let (done, total) = count_all_checklist(&task_dir);
        assert_eq!(done, 3);
        assert_eq!(total, 5);
    }
}
