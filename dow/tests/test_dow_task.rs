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
            "--file",
            r#"{"modify":["src/login.rs"],"test":["tests/login.rs"]}"#,
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
fn test_task_create_reports_all_missing_cli_fields() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args(["task", "create", "--title", "Incomplete task"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    for expected in [
        "--type",
        "--priority",
        "--refs",
        "--file",
        "--depends-on",
        "--complexity",
        "--done-when",
    ] {
        assert!(
            stderr.contains(expected),
            "missing {} in: {}",
            expected,
            stderr
        );
    }
    assert!(stderr.contains("dow task schema"));
    let branch = default_branch(&dir);
    assert_eq!(
        fs::read_dir(dir.join(".dev-doc").join(branch).join("task"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn test_task_create_json_reports_all_batch_errors() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"[
        {"title":"missing fields"},
        {"title":"invalid values","type":"unknown","priority":"P3","refs":"","files":{"modify":[]},"depends_on":[],"parallel":false,"complexity":"X","done_when":[]}
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

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[0].type"), "stderr: {}", stderr);
    assert!(stderr.contains("[1].type"), "stderr: {}", stderr);
    assert!(stderr.contains("[1].priority"), "stderr: {}", stderr);
    assert!(stderr.contains("[1].complexity"), "stderr: {}", stderr);
    assert!(stderr.contains("[1].files"));
    let branch = default_branch(&dir);
    assert_eq!(
        fs::read_dir(dir.join(".dev-doc").join(branch).join("task"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn test_nested_project_uses_nearest_workflow_root() {
    let parent = tempfile::tempdir().unwrap();
    common::git_init_with_commit(parent.path());

    let branch = default_branch(parent.path());
    let parent_doc = parent.path().join(".dev-doc").join(&branch);
    fs::create_dir_all(parent_doc.join("task")).unwrap();
    fs::create_dir_all(parent_doc.join("issue")).unwrap();
    fs::write(
        parent_doc.join("STATUS.yaml"),
        "name: parent-project\nphase: DEV\nmode: fast\nupdated: 2026-08-02 10:00\nstarted: 2026-08-02 09:00\n",
    )
    .unwrap();
    let parent_task = parent_doc.join("task/task_2026-08-02_1.md");
    fs::write(
        &parent_task,
        "---\ntitle: TASK - parent\nnums: 1\n---\n\n- [ ] TASK-T001: Parent task\n  - type: feat\n  - priority: P1\n",
    )
    .unwrap();

    let child = parent.path().join("tmp/rozsa-rt");
    fs::create_dir_all(&child).unwrap();

    let init = Command::new(dow_cmd())
        .args(["init", "--name", "rozsa-rt", "--mode", "fast"])
        .current_dir(&child)
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    let child_doc = child.join(".dev-doc").join(&branch);
    assert!(child_doc.join("STATUS.yaml").exists());
    assert!(child.join("VERSION").exists());
    assert!(!parent.path().join("VERSION").exists());
    assert!(!parent.path().join("docs").exists());

    let create = Command::new(dow_cmd())
        .args([
            "task",
            "create",
            "--title",
            "Child task",
            "--task-type",
            "fix",
            "--priority",
            "P1",
            "--refs",
            "user-request",
            "--file",
            r#"{"modify":["src/child.rs"]}"#,
            "--depends-on",
            "",
            "--complexity",
            "S",
            "--done-when",
            "child task exists",
        ])
        .current_dir(&child)
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&create.stdout).trim(), "TASK-T001");
    assert!(fs::read_to_string(&parent_task)
        .unwrap()
        .contains("TASK-T001: Parent task"));

    let list = Command::new(dow_cmd())
        .args(["task", "list"])
        .current_dir(&child)
        .output()
        .unwrap();
    assert!(list.status.success());
    let items: Vec<serde_json::Value> = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "TASK-T001");
    assert_eq!(items[0]["title"], "Child task");

    let status = Command::new(dow_cmd())
        .args(["status"])
        .current_dir(&child)
        .output()
        .unwrap();
    assert!(status.status.success());
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status_json["name"], "rozsa-rt");
    let expected_doc_root = fs::canonicalize(&child_doc).unwrap();
    assert_eq!(
        status_json["doc_root"].as_str().unwrap(),
        expected_doc_root.to_string_lossy().as_ref()
    );

    let claim = Command::new(dow_cmd())
        .args(["claim", "T001"])
        .current_dir(&child)
        .output()
        .unwrap();
    assert!(
        claim.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&claim.stderr)
    );
    assert!(child_doc.join("claim.lock").exists());
    assert!(!parent_doc.join("claim.lock").exists());
}

#[test]
fn test_task_create_with_stdin_json_object() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"{"title": "Add auth middleware", "type": "feat", "priority": "P1", "refs": "", "files": {"modify": ["src/auth.rs"]}, "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["middleware added"]}"#;

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
        {"title": "Task A", "type": "feat", "priority": "P0", "refs": "", "files": {"modify": ["src/a.rs"]}, "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["done"]},
        {"title": "Task B", "type": "fix", "priority": "P1", "refs": "", "files": {"create": ["src/b.rs"]}, "depends_on": [], "parallel": false, "complexity": "S", "done_when": ["done"]}
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
            "--file",
            r#"{"modify":["src/new.rs"]}"#,
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
            "--file",
            r#"{"modify":["src/oversized.rs"]}"#,
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

#[test]
fn test_task_create_rejects_unscoped_files() {
    let dir = create_test_dir();
    setup_env(&dir);

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "create",
            "--title",
            "test-only scope",
            "--task-type",
            "test",
            "--priority",
            "P1",
            "--refs",
            "user-request",
            "--file",
            r#"{"test":["tests/task.rs"]}"#,
            "--depends-on",
            "",
            "--complexity",
            "S",
            "--done-when",
            "test scope",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("files.create"));
    let branch = default_branch(&dir);
    assert_eq!(
        fs::read_dir(dir.join(".dev-doc").join(branch).join("task"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn test_task_create_rejects_legacy_flat_json() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"{"title":"legacy","type":"feat","priority":"P1","refs":"","files_modify":["src/old.rs"],"files_create":[],"files_test":[],"depends_on":[],"parallel":false,"complexity":"S","done_when":["rejected"]}"#;
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

    assert!(!output.status.success());
    let branch = default_branch(&dir);
    assert_eq!(
        fs::read_dir(dir.join(".dev-doc").join(branch).join("task"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn test_task_create_rejects_mixed_cli_and_stdin() {
    let dir = create_test_dir();
    setup_env(&dir);

    let json_input = r#"{"title":"stdin task","type":"feat","priority":"P1","refs":"","files":{"modify":["src/a.rs"]},"depends_on":[],"parallel":false,"complexity":"S","done_when":["done"]}"#;
    let mut child = Command::new(dow_cmd())
        .args(["task", "create", "--title", "cli task"])
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

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot combine"));
}

#[test]
fn test_task_update_json_reports_unknown_and_enum_errors() {
    let dir = create_test_dir();
    setup_env(&dir);
    let branch = default_branch(&dir);
    let task_path = dir
        .join(".dev-doc")
        .join(&branch)
        .join("task/task_2026-06-25_1.md");
    let original = "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Keep task\n  - type: feat\n  - priority: P1\n  - files:\n      create: []\n      modify: [old.rs]\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - unchanged\n";
    fs::write(&task_path, original).unwrap();

    let mut child = Command::new(dow_cmd())
        .args(["task", "update", "TASK-T001"])
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
        .write_all(br#"{"type":"unknown","priority":"P3","complexity":"X","unexpected":true}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown field"), "stderr: {}", stderr);
    assert!(stderr.contains("Invalid type"), "stderr: {}", stderr);
    assert!(stderr.contains("Invalid priority"), "stderr: {}", stderr);
    assert!(stderr.contains("Invalid complexity"), "stderr: {}", stderr);
    assert_eq!(fs::read_to_string(task_path).unwrap(), original);
}

#[test]
fn test_task_update_nested_files_incremental() {
    let dir = create_test_dir();
    setup_env(&dir);
    let branch = default_branch(&dir);
    let task_path = dir
        .join(".dev-doc")
        .join(&branch)
        .join("task/task_2026-06-25_1.md");
    fs::write(
        &task_path,
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Update scope\n  - type: feat\n  - priority: P1\n  - files:\n      create: []\n      modify: [old.rs]\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - updated\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "update",
            "TASK-T001",
            "--file",
            r#"{"create":["src/new.rs"],"modify":["+new.rs","-old.rs"]}"#,
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = fs::read_to_string(task_path).unwrap();
    assert!(content.contains("create: [\"src/new.rs\"]"));
    assert!(content.contains("modify: [\"new.rs\"]"));
    assert!(!content.contains("modify: [\"old.rs\"]"));
}

#[test]
fn test_task_update_rejects_removing_last_scope() {
    let dir = create_test_dir();
    setup_env(&dir);
    let branch = default_branch(&dir);
    let task_path = dir
        .join(".dev-doc")
        .join(&branch)
        .join("task/task_2026-06-25_1.md");
    let original = "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Keep scope\n  - type: feat\n  - priority: P1\n  - files:\n      create: []\n      modify: [old.rs]\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - unchanged\n";
    fs::write(&task_path, original).unwrap();

    let output = Command::new(dow_cmd())
        .args([
            "task",
            "update",
            "TASK-T001",
            "--file",
            r#"{"modify":["-old.rs"]}"#,
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("last files"));
    assert_eq!(fs::read_to_string(task_path).unwrap(), original);
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

// ─── Short ID Tests ─────────────────────────────────────────────────────────

#[test]
fn test_task_show_accepts_short_id() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-07-22_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Short ID test\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - works\n",
    )
    .unwrap();

    // T001 (short with padding)
    let output = Command::new(dow_cmd())
        .args(["task", "show", "T001"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "T001 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["id"], "TASK-T001");

    // T1 (short without padding)
    let output = Command::new(dow_cmd())
        .args(["task", "show", "T1"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "T1 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["id"], "TASK-T001");
}

#[test]
fn test_task_done_accepts_short_id() {
    let dir = create_test_dir();
    setup_env(&dir);

    let branch = default_branch(&dir);
    let task_dir = dir.join(".dev-doc").join(&branch).join("task");
    fs::write(
        task_dir.join("task_2026-07-22_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: Done short\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - works\n",
    )
    .unwrap();

    let output = Command::new(dow_cmd())
        .args(["task", "done", "T1"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "T1 done failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
