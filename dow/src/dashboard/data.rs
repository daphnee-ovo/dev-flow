use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ProjectData {
    pub status: StatusData,
    pub tasks: Vec<TaskData>,
    pub issues: Vec<IssueData>,
    pub docs: DocsData,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatusData {
    pub name: String,
    pub phase: String,
    pub mode: String,
    pub version: String,
    pub goals_minor: String,
    pub updated: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TaskData {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub complexity: String,
    pub status: String,
    pub depends_on: Vec<String>,
    pub done_when: Vec<String>,
    pub r#type: String,
    pub refs: String,
    pub files_create: Vec<String>,
    pub files_modify: Vec<String>,
    pub files_test: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct IssueData {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub status: String,
    pub description: String,
    pub files_modify: Vec<String>,
    pub files_create: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DocsData {
    pub brainstorm: DocEntry,
    pub prd: DocEntry,
    pub spec: DocEntry,
}

#[derive(Debug, Serialize, Clone)]
pub struct DocEntry {
    pub exists: bool,
    pub content: Option<String>,
}

pub fn collect_project_data(doc_root: &Path) -> ProjectData {
    ProjectData {
        status: read_status(doc_root),
        tasks: read_tasks(doc_root),
        issues: read_issues(doc_root),
        docs: read_docs(doc_root),
    }
}

fn read_status(doc_root: &Path) -> StatusData {
    let path = doc_root.join("STATUS.yaml");
    let content = fs::read_to_string(&path).unwrap_or_default();

    let get = |key: &str| -> String {
        content
            .lines()
            .find(|l| l.starts_with(&format!("{}:", key)))
            .map(|l| l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_default()
    };

    let version = read_version();

    StatusData {
        name: get("name"),
        phase: get("phase"),
        mode: get("mode"),
        version,
        goals_minor: get("goals_minor"),
        updated: get("updated"),
    }
}

fn read_version() -> String {
    crate::core::version::read_current().unwrap_or_default()
}

fn read_tasks(doc_root: &Path) -> Vec<TaskData> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return vec![];
    }

    let mut tasks = Vec::new();

    let mut files: Vec<_> = fs::read_dir(&task_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for entry in files {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        tasks.extend(parse_tasks_from_file(&content));
    }

    tasks
}

fn parse_tasks_from_file(content: &str) -> Vec<TaskData> {
    let mut tasks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let (is_done, id, title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            if let Some(colon_pos) = rest.find(':') {
                (false, rest[..colon_pos].trim().to_string(), rest[colon_pos + 1..].trim().to_string())
            } else {
                continue;
            }
        } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            if let Some(colon_pos) = rest.find(':') {
                (true, rest[..colon_pos].trim().to_string(), rest[colon_pos + 1..].trim().to_string())
            } else {
                continue;
            }
        } else {
            continue;
        };

        if !id.starts_with("TASK-T") {
            continue;
        }

        let mut task_type = String::new();
        let mut priority = String::new();
        let mut complexity = String::new();
        let mut refs = String::new();
        let mut depends_on = Vec::new();
        let mut done_when = Vec::new();
        let mut files_create = Vec::new();
        let mut files_modify = Vec::new();
        let mut files_test = Vec::new();
        let mut in_done_when = false;
        let mut in_files = false;

        for j in (i + 1)..lines.len() {
            let sub = lines[j];
            let sub_trimmed = sub.trim();

            if sub_trimmed.starts_with("- [ ]") || sub_trimmed.starts_with("- [x]") {
                break;
            }

            if in_done_when {
                if sub.starts_with("      - ") || sub.starts_with("      -") {
                    done_when.push(sub.trim().trim_start_matches("- ").to_string());
                    continue;
                }
                in_done_when = false;
            }

            if in_files {
                if sub_trimmed.starts_with("create:") {
                    files_create = parse_inline_list(sub_trimmed.strip_prefix("create:").unwrap());
                    continue;
                } else if sub_trimmed.starts_with("modify:") {
                    files_modify = parse_inline_list(sub_trimmed.strip_prefix("modify:").unwrap());
                    continue;
                } else if sub_trimmed.starts_with("test:") {
                    files_test = parse_inline_list(sub_trimmed.strip_prefix("test:").unwrap());
                    continue;
                } else if !sub_trimmed.is_empty() && !sub.starts_with("      ") {
                    in_files = false;
                }
            }

            if sub_trimmed.starts_with("- type:") {
                task_type = sub_trimmed.strip_prefix("- type:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- priority:") {
                priority = sub_trimmed.strip_prefix("- priority:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- refs:") {
                refs = sub_trimmed.strip_prefix("- refs:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- files:") {
                in_files = true;
            } else if sub_trimmed.starts_with("- complexity:") {
                complexity = sub_trimmed.strip_prefix("- complexity:").unwrap().trim().to_string();
            } else if sub_trimmed.starts_with("- depends_on:") {
                depends_on = parse_inline_list(sub_trimmed.strip_prefix("- depends_on:").unwrap());
            } else if sub_trimmed.starts_with("- done_when:") {
                in_done_when = true;
            }
        }

        tasks.push(TaskData {
            id,
            title,
            priority,
            complexity,
            status: if is_done { "done".to_string() } else { "pending".to_string() },
            depends_on,
            done_when,
            r#type: task_type,
            refs,
            files_create,
            files_modify,
            files_test,
        });
    }

    tasks
}

fn read_issues(doc_root: &Path) -> Vec<IssueData> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return vec![];
    }

    let mut issues = Vec::new();

    let mut files: Vec<_> = fs::read_dir(&issue_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for entry in files {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        issues.extend(parse_issues_from_file(&content));
    }

    issues
}

/// Split "ISSUE-I011：title" or "ISSUE-I011: title" handling both ASCII and full-width colons
fn split_id_title(rest: &str, prefix: &str) -> Option<(String, String)> {
    // Try ASCII colon first, then full-width
    for sep in &[":", "："] {
        if let Some(pos) = rest.find(sep) {
            let id = rest[..pos].trim().to_string();
            if !id.starts_with(prefix) {
                return None;
            }
            let title = rest[pos + sep.len()..].trim().to_string();
            return Some((id, title));
        }
    }
    None
}

fn parse_issues_from_file(content: &str) -> Vec<IssueData> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let (is_closed, id, title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            if let Some((id, title)) = split_id_title(rest, "ISSUE-") {
                (false, id, title)
            } else {
                continue;
            }
        } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            if let Some((id, title)) = split_id_title(rest, "ISSUE-") {
                (true, id, title)
            } else {
                continue;
            }
        } else {
            continue;
        };

        let mut severity = String::new();
        let mut description = String::new();
        let mut files_modify = Vec::new();
        let mut files_create = Vec::new();

        for j in (i + 1)..lines.len() {
            let sub = lines[j].trim();
            if sub.starts_with("- [ ]") || sub.starts_with("- [x]") {
                break;
            }
            if sub.starts_with("- severity:") {
                severity = sub.strip_prefix("- severity:").unwrap().trim().to_string();
            } else if sub.starts_with("- description:") {
                description = sub.strip_prefix("- description:").unwrap().trim().to_string();
            } else if sub.starts_with("- description：") {
                description = sub.strip_prefix("- description：").unwrap().trim().to_string();
            } else if sub.starts_with("- files_modify:") {
                files_modify = parse_inline_list(sub.strip_prefix("- files_modify:").unwrap());
            } else if sub.starts_with("- files_create:") {
                files_create = parse_inline_list(sub.strip_prefix("- files_create:").unwrap());
            }
        }

        issues.push(IssueData {
            id,
            title,
            severity,
            status: if is_closed { "closed".to_string() } else { "open".to_string() },
            description,
            files_modify,
            files_create,
        });
    }

    issues
}

fn read_docs(doc_root: &Path) -> DocsData {
    DocsData {
        brainstorm: read_doc_entry(doc_root, "BRAINSTORM.md"),
        prd: read_doc_entry(doc_root, "PRD.md"),
        spec: read_doc_entry(doc_root, "SPEC.md"),
    }
}

fn read_doc_entry(doc_root: &Path, filename: &str) -> DocEntry {
    let path = doc_root.join(filename);
    if path.exists() {
        DocEntry {
            exists: true,
            content: fs::read_to_string(&path).ok(),
        }
    } else {
        DocEntry {
            exists: false,
            content: None,
        }
    }
}

fn parse_inline_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if s == "[]" || s.is_empty() {
        return vec![];
    }
    let s = s.trim_start_matches('[').trim_end_matches(']');
    s.split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;

    /// Restore the process cwd when dropped (tests that chdir into a tempdir).
    struct CwdGuard {
        original: PathBuf,
    }
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    #[test]
    #[serial]
    fn test_collect_project_data() {
        let dir = tempfile::tempdir().unwrap();
        let doc_root = dir.path();

        // Make the tempdir a git repo and chdir into it so doc_root::project_root()
        // (git rev-parse --show-toplevel) resolves here, letting read_current()
        // find the VERSION file written below. #[serial] avoids concurrent cwd races.
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(doc_root)
            .status()
            .expect("git init failed");
        let _cwd_guard = CwdGuard {
            original: std::env::current_dir().unwrap(),
        };
        std::env::set_current_dir(doc_root).unwrap();

        // STATUS.yaml
        fs::write(
            doc_root.join("STATUS.yaml"),
            "name: test-project\nphase: DEV\nmode: fast\ngoals_minor: testing\nupdated: 2026-01-01\n",
        ).unwrap();

        // task/
        fs::create_dir_all(doc_root.join("task")).unwrap();
        fs::write(
            doc_root.join("task/task_2026-01-01_1.md"),
            r#"---
title: test batch
nums: 2
---

- [ ] TASK-T001: First task
  - type: feat
  - priority: P0
  - complexity: M
  - depends_on: []
  - done_when:
      - criterion one
      - criterion two
- [x] TASK-T002: Second task
  - type: fix
  - priority: P1
  - complexity: S
  - depends_on: ["TASK-T001"]
  - done_when:
      - done criterion
"#,
        ).unwrap();

        // BRAINSTORM.md
        fs::write(doc_root.join("BRAINSTORM.md"), "# Brainstorm\nContent here").unwrap();

        // VERSION at project_root (== doc_root after chdir): entry for the branch
        // read_current() will resolve (env-independent — matches whatever the
        // fresh git repo's initial branch is).
        let branch = crate::core::version::resolve_branch();
        fs::write(
            doc_root.join("VERSION"),
            format!("({})1.0.0\n", branch),
        )
        .unwrap();

        let data = collect_project_data(doc_root);

        assert_eq!(data.status.name, "test-project");
        assert_eq!(data.status.phase, "DEV");
        assert_eq!(data.status.mode, "fast");
        assert_eq!(data.status.version, "1.0.0");

        assert_eq!(data.tasks.len(), 2);
        assert_eq!(data.tasks[0].id, "TASK-T001");
        assert_eq!(data.tasks[0].status, "pending");
        assert_eq!(data.tasks[0].priority, "P0");
        assert_eq!(data.tasks[0].complexity, "M");
        assert_eq!(data.tasks[0].done_when.len(), 2);
        assert_eq!(data.tasks[1].id, "TASK-T002");
        assert_eq!(data.tasks[1].status, "done");
        assert_eq!(data.tasks[1].depends_on, vec!["TASK-T001"]);

        assert!(data.docs.brainstorm.exists);
        assert!(data.docs.brainstorm.content.unwrap().contains("Brainstorm"));
        assert!(!data.docs.prd.exists);
        assert!(!data.docs.spec.exists);
    }

    #[test]
    fn test_parse_issues_fullwidth_colon() {
        let content = r#"---
source: other
nums: 1
---

- [ ] ISSUE-I011：dashboard 依赖图不显示 issue 节点
  - severity: P1
  - description：依赖图只展示 task 节点
- [x] ISSUE-I012: fixed issue
  - severity: P0
  - description: already fixed
"#;
        let issues = parse_issues_from_file(content);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "ISSUE-I011");
        assert_eq!(issues[0].title, "dashboard 依赖图不显示 issue 节点");
        assert_eq!(issues[0].severity, "P1");
        assert_eq!(issues[0].status, "open");
        assert_eq!(issues[0].description, "依赖图只展示 task 节点");
        assert_eq!(issues[1].id, "ISSUE-I012");
        assert_eq!(issues[1].status, "closed");
    }
}
