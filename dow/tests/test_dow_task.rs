// tests/
// ├── test_dow_task.rs  -- dow task 子命令集成测试

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use common::default_branch;

static TEST_SEQ: AtomicU32 = AtomicU32::new(0);

fn create_test_dir() -> PathBuf {
    let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp/test_task");
    let dir = base.join(format!("t{}", seq));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_env(dir: &Path) {
    common::git_init_with_commit(dir);
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-06-25 10:00\nstarted: 2026-06-25 09:00\n",
    )
    .unwrap();
}

fn dow_cmd() -> String {
    env!("CARGO_BIN_EXE_dow").to_string()
}

// ─── Create Tests ────────────────────────────────────────────────────────────

#[test]
fn test_task_create_with_flags() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "create",
            "--title",
            "Implement login flow",
            "--task-type",
            "feat",
            "--priority",
            "P0",
            "--refs",
            "user-request",
            "--files-modify",
            "",
            "--files-create",
            "",
            "--files-test",
            "",
            "--depends-on",
            "",
            "--complexity",
            "M",
            "--done-when",
            "login works,tests pass",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Successful creation returns the generated task ID.
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "TASK-T001");

    // Verify file was created
    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    let files: Vec<_> = fs::read_dir(&task_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("task_"))
        .collect();
    assert_eq!(files.len(), 1);

    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("TASK-T001: Implement login flow"));
    assert!(content.contains("- type: feat"));
    assert!(content.contains("- priority: P0"));
    assert!(content.contains("- complexity: M"));
    assert!(content.contains("- login works"));
    assert!(content.contains("- tests pass"));
}

#[test]
fn test_task_create_with_stdin_json_object() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"{"title": "Add auth middleware", "type": "feat", "priority": "P1", "refs": "", "files_modify": [], "files_create": [], "files_test": [], "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["middleware added"]}"#;

    let mut child = Command::new(dow_cmd())
        .args(["task", "create"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    let files: Vec<_> = fs::read_dir(&task_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("task_"))
        .collect();
    assert_eq!(files.len(), 1);
    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("TASK-T001: Add auth middleware"));
}

#[test]
fn test_task_create_with_stdin_json_array() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"[
        {"title": "Task A", "type": "feat", "priority": "P0", "refs": "", "files_modify": [], "files_create": [], "files_test": [], "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["done"]},
        {"title": "Task B", "type": "fix", "priority": "P1", "refs": "", "files_modify": [], "files_create": [], "files_test": [], "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["done"]}
    ]"#;

    let mut child = Command::new(dow_cmd())
        .args(["task", "create"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    let files: Vec<_> = fs::read_dir(&task_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("task_"))
        .collect();
    assert_eq!(files.len(), 1);
    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("TASK-T001: Task A"));
    assert!(content.contains("TASK-T002: Task B"));
    assert!(content.contains("nums: 2"));
}

#[test]
fn test_task_create_increments_id() {
    let dir = create_test_dir();
    setup_env(&dir);

    // Pre-populate a task file
    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-24_1.md"),
        "---\ntitle: TASK - old batch\nnums: 2\n---\n\n- [x] TASK-T001: Old task A\n  - type: feat\n  - priority: P0\n- [ ] TASK-T002: Old task B\n  - type: fix\n  - priority: P1\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "create",
            "--title",
            "New task",
            "--task-type",
            "feat",
            "--priority",
            "P1",
            "--refs",
            "",
            "--files-modify",
            "",
            "--files-create",
            "",
            "--files-test",
            "",
            "--depends-on",
            "",
            "--complexity",
            "S",
            "--done-when",
            "done",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should create TASK-T003 in today's batch file
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let files: Vec<_> = fs::read_dir(&task_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&format!("task_{}", today))
        })
        .collect();
    assert_eq!(files.len(), 1);
    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("TASK-T003: New task"));
}

#[test]
fn test_task_create_rejects_xl_complexity() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "create",
            "--title",
            "Oversized task",
            "--task-type",
            "feat",
            "--priority",
            "P1",
            "--refs",
            "user-request",
            "--files-modify",
            "",
            "--files-create",
            "",
            "--files-test",
            "",
            "--depends-on",
            "",
            "--complexity",
            "XL",
            "--done-when",
            "split into smaller tasks",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("valid: S/M/L"));
    assert!(stderr.contains("split oversized work into multiple Tasks"));
    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    assert_eq!(fs::read_dir(task_dir).unwrap().count(), 0);
}

#[test]
fn test_task_create_no_input_exits_2() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args(["task", "create"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
}

// ─── List Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_task_list_pending_only() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 3\n---\n\n- [ ] TASK-T001: Pending one\n  - type: feat\n  - priority: P0\n- [x] TASK-T002: Done one\n  - type: fix\n  - priority: P1\n- [ ] TASK-T003: Pending two\n  - type: refactor\n  - priority: P2\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "list"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "TASK-T001");
    assert_eq!(items[0]["status"], "pending");
    assert_eq!(items[1]["id"], "TASK-T003");
}

#[test]
fn test_task_list_all() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 2\n---\n\n- [ ] TASK-T001: Pending\n  - type: feat\n  - priority: P0\n- [x] TASK-T002: Done\n  - type: fix\n  - priority: P1\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "list", "--all"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[1]["status"], "done");
}

#[test]
fn test_task_list_includes_done_files() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("done_task_2026-06-24_1.md"),
        "---\ntitle: TASK - old\nnums: 1\n---\n\n- [x] TASK-T001: Archived\n  - type: feat\n  - priority: P0\n",
    )
    .unwrap();

    // --all should include tasks from done_ files
    let output = Command::new(dow_cmd())
        .args(["task", "list", "--all"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "TASK-T001");
    assert_eq!(items[0]["status"], "done");
}

// ─── Show Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_task_show() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-25_1.md"),
        r#"---
title: TASK - batch
nums: 1
---

- [ ] TASK-T001: Implement login
  - type: feat
  - priority: P0
  - refs: SPEC-AC-001
  - files:
      create: ["src/auth.rs"]
      modify: ["src/main.rs"]
      test: ["tests/auth_test.rs"]
  - depends_on: []
  - parallel: false
  - complexity: M
  - done_when:
      - login endpoint works
      - tests pass
"#,
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "show", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["id"], "TASK-T001");
    assert_eq!(json["title"], "Implement login");
    assert_eq!(json["type"], "feat");
    assert_eq!(json["priority"], "P0");
    assert_eq!(json["refs"], "SPEC-AC-001");
    assert_eq!(json["status"], "pending");
    assert_eq!(json["complexity"], "M");
    assert_eq!(json["parallel"], false);
    assert_eq!(json["files"]["create"][0], "src/auth.rs");
    assert_eq!(json["files"]["modify"][0], "src/main.rs");
    assert_eq!(json["files"]["test"][0], "tests/auth_test.rs");
    assert_eq!(json["done_when"][0], "login endpoint works");
    assert_eq!(json["done_when"][1], "tests pass");
}

#[test]
fn test_task_show_not_found() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args(["task", "show", "TASK-T999"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ─── Done Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_task_done_marks_complete() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 2\n---\n\n- [ ] TASK-T001: First\n  - type: feat\n  - priority: P0\n- [ ] TASK-T002: Second\n  - type: fix\n  - priority: P1\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "silent on success");

    // Verify content changed
    let content = fs::read_to_string(task_dir.join("task_2026-06-25_1.md")).unwrap();
    assert!(content.contains("- [x] TASK-T001:"));
    assert!(content.contains("- [ ] TASK-T002:"));
}

#[test]
fn test_task_done_renames_when_all_done() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Only task\n  - type: feat\n  - priority: P0\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());

    // Original file should not exist
    assert!(!task_dir.join("task_2026-06-25_1.md").exists());
    // done_ prefixed file should exist
    assert!(task_dir.join("done_task_2026-06-25_1.md").exists());

    let content = fs::read_to_string(task_dir.join("done_task_2026-06-25_1.md")).unwrap();
    assert!(content.contains("- [x] TASK-T001:"));
}

#[test]
fn test_task_done_blocks_on_test_failure_and_creates_issue() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        dir.join("fail.sh"),
        "#!/bin/sh\necho 'raw task test failure' >&2\nexit 1\n",
    )
    .unwrap();
    let task_path = task_dir.join("task_2026-06-25_1.md");
    let original = "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Failing task\n  - type: feat\n  - priority: P0\n  - files:\n      test: [\"fail.sh\"]\n";
    fs::write(&task_path, original).unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("raw task test failure"));
    assert_eq!(fs::read_to_string(&task_path).unwrap(), original);
    let issue_dir = dir.join(".dev-doc").join(&branch).join("issue");
    let issue_files = fs::read_dir(issue_dir)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    assert_eq!(issue_files.len(), 1);
    let issue = fs::read_to_string(issue_files[0].path()).unwrap();
    assert!(issue.contains("Test TASK-T001 fail:"));
    assert!(issue.contains("raw task test failure"));
}

#[test]
fn test_task_done_blocks_on_test_precondition_without_issue() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    let task_path = task_dir.join("task_2026-06-25_1.md");
    let original = "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Unsupported test\n  - type: feat\n  - priority: P0\n  - files:\n      test: [\"unsupported.txt\"]\n";
    fs::write(&task_path, original).unwrap();
    fs::write(dir.join("unsupported.txt"), "not a test").unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No built-in test adapter"));
    assert_eq!(fs::read_to_string(&task_path).unwrap(), original);
    let issue_dir = dir.join(".dev-doc").join(&branch).join("issue");
    assert_eq!(fs::read_dir(issue_dir).unwrap().count(), 0);
}

#[test]
fn test_task_done_multi_ids_keeps_prior_success_when_later_test_fails() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        dir.join("fail.sh"),
        "#!/bin/sh\necho 'second task failed' >&2\nexit 1\n",
    )
    .unwrap();
    let task_path = task_dir.join("task_2026-06-25_1.md");
    fs::write(
        &task_path,
        "---\ntitle: TASK - batch\nnums: 2\n---\n\n- [ ] TASK-T001: First task\n  - type: feat\n  - priority: P0\n- [ ] TASK-T002: Second task\n  - type: feat\n  - priority: P0\n  - files:\n      test: [\"fail.sh\"]\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T001", "TASK-T002"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let content = fs::read_to_string(&task_path).unwrap();
    assert!(content.contains("- [x] TASK-T001:"));
    assert!(content.contains("- [ ] TASK-T002:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("second task failed"));
}

#[test]
fn test_task_done_not_found() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args(["task", "done", "TASK-T999"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ─── Reopen Tests ────────────────────────────────────────────────────────────

#[test]
fn test_task_reopen_without_confirm_shows_impact() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("done_task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [x] TASK-T001: Completed task\n  - type: feat\n  - priority: P0\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "reopen", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["id"], "TASK-T001");
    assert!(json["confirm_token"].as_str().unwrap().starts_with("TRO-"));
    assert_eq!(json["confirm_token"].as_str().unwrap().len(), 10);
}

#[test]
fn test_task_reopen_with_confirm() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("done_task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [x] TASK-T001: Completed task\n  - type: feat\n  - priority: P0\n",
    )
    .unwrap();

    // First get the token
    let output = Command::new(dow_cmd())
        .args(["task", "reopen", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let token = json["confirm_token"].as_str().unwrap().to_string();

    // Then confirm with the token
    let output = Command::new(dow_cmd())
        .args(["task", "reopen", "TASK-T001", "--confirm", &token])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "silent on success");

    // File should be renamed back (done_ prefix removed)
    assert!(!task_dir.join("done_task_2026-06-25_1.md").exists());
    assert!(task_dir.join("task_2026-06-25_1.md").exists());

    let content = fs::read_to_string(task_dir.join("task_2026-06-25_1.md")).unwrap();
    assert!(content.contains("- [ ] TASK-T001:"));
}

#[test]
fn test_task_reopen_invalid_token() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("done_task_2026-06-25_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [x] TASK-T001: Completed task\n  - type: feat\n  - priority: P0\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "reopen", "TASK-T001", "--confirm", "TRO-000000"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
}

// ─── Schema Test ─────────────────────────────────────────────────────────────

#[test]
fn test_task_schema_outputs_json() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args(["task", "schema"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(json["fields"]["id"].is_object());
    assert!(json["fields"]["title"].is_object());
    assert!(json["fields"]["type"].is_object());
    assert!(json["fields"]["priority"].is_object());
    assert_eq!(
        json["fields"]["complexity"]["enum"],
        serde_json::json!(["S", "M", "L"])
    );
    assert!(json["file_format"]["name_pattern"].is_string());
}
