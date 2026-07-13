mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::default_branch;

fn setup_project(dir: &Path, ci: &str) -> std::path::PathBuf {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: fast\nupdated: 2026-07-12 10:00\nstarted: 2026-07-12 10:00\n",
    )
    .unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    fs::write(dir.join(".dev-doc/test.ci"), ci).unwrap();
    doc
}

#[test]
fn test_full_ci_runs_all_commands() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_project(
        dir.path(),
        "test:\n  run: printf first\n  run: printf second\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PASS");
    assert_eq!(json["total"], 2);
    assert_eq!(json["passed"], 2);
}

#[test]
fn test_task_ci_expands_task_placeholders() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(
        dir.path(),
        "devtest:\n  run: test -n {{task_id}} && test -f {{test_files}}\n",
    );
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("tests/task.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: config task\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/task.sh]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - config works\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["test", "TASK-T001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["target"], "TASK-T001");
    assert_eq!(json["outcome"], "PASS");
}

#[test]
fn test_empty_task_files_passes() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path(), "");
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: empty task\n  - type: docs\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - docs work\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["test", "TASK-T001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PASS");
    assert_eq!(json["total"], 0);
}

#[test]
fn test_invalid_ci_is_precondition_failure() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_project(dir.path(), "test:\n  invalid: value\n");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PRECONDITION_FAILED");
    assert_eq!(json["precondition_failed"], 1);
    let issue_files: Vec<_> = fs::read_dir(
        dir.path()
            .join(".dev-doc")
            .join(default_branch(dir.path()))
            .join("issue"),
    )
    .unwrap()
    .flatten()
    .collect();
    assert!(issue_files.is_empty());
}

#[test]
fn test_failed_ci_creates_existing_issue_shape() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(
        dir.path(),
        "test:\n  run: printf 'raw failure' >&2; exit 3\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "TEST_FAILED");
    let issue_file = fs::read_dir(doc.join("issue"))
        .unwrap()
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("issue_test_")
        })
        .unwrap();
    let issue = fs::read_to_string(issue_file.path()).unwrap();
    assert!(issue.contains("Test fail:raw failure"));
    assert!(issue.contains("raw failure"));
    assert!(issue.contains("source: test"));
}
