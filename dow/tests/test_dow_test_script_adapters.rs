mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::default_branch;

fn setup_project(dir: &Path) -> std::path::PathBuf {
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
        "name: fixture\nphase: DEV\nmode: fast\nupdated: 2026-07-12 10:00\nstarted: 2026-07-12 10:00\n",
    )
    .unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    doc
}

fn run_dow(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn test_javascript_full_uses_package_test_script() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_project(dir.path());
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager":"npm@10","scripts":{"test":"node --test tests/node_test.js"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/node_test.js"),
        "const test = require('node:test'); const assert = require('node:assert'); test('ok', () => assert.equal(1, 1));\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["test"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PASS");
    assert_eq!(json["total"], 1);
}

#[test]
fn test_javascript_task_uses_node_runner() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path());
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager":"npm@10"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/node_test.js"),
        "const test = require('node:test'); const assert = require('node:assert'); test('ok', () => assert.equal(1, 1));\n",
    )
    .unwrap();
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: node test\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/node_test.js]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - node test works\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["test", "TASK-T001"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_typescript_without_known_runner_is_precondition_failure() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path());
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager":"npm@10"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/example.ts"),
        "export const value = 1;\n",
    )
    .unwrap();
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: unknown ts runner\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/example.ts]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - runner is required\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["test", "TASK-T001"]);
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PRECONDITION_FAILED");
}

#[test]
fn test_python_without_interpreter_is_precondition_failure() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_project(dir.path());
    fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/test_example.py"),
        "def test_ok():\n    assert True\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["test"]);
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PRECONDITION_FAILED");
    assert_eq!(json["precondition_failed"], 1);
}
