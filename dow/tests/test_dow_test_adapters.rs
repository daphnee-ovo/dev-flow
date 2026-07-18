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

    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n#[cfg(test)] mod tests { use crate::answer; #[test] fn answer_is_ok() { assert_eq!(answer(), 42); } }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(
        dir.join("tests/integration_test.rs"),
        "#[test] fn integration_is_ok() { assert_eq!(2 + 2, 4); }\n",
    )
    .unwrap();
    fs::write(dir.join("tests/test_shell.sh"), "#!/bin/bash\nexit 0\n").unwrap();
    fs::write(dir.join("tests/test_all.sh"), "#!/bin/bash\nexit 1\n").unwrap();

    fs::write(
        dir.join("go.mod"),
        "module example.com/fixture\n\ngo 1.22\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("pkg")).unwrap();
    fs::write(
        dir.join("pkg/pkg.go"),
        "package pkg\n\nfunc Answer() int { return 42 }\n",
    )
    .unwrap();
    fs::write(
        dir.join("pkg/pkg_test.go"),
        "package pkg\n\nimport \"testing\"\n\nfunc TestAnswer(t *testing.T) { if Answer() != 42 { t.Fatal(\"bad answer\") } }\n",
    )
    .unwrap();

    doc
}

fn run_dow(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .env("GOCACHE", dir.join("go-cache"))
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn test_full_adapters_run_native_commands_and_skip_aggregator() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_project(dir.path());

    let output = run_dow(dir.path(), &["test"]);

    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PASS");
    assert_eq!(json["total"], 3);
    assert_eq!(json["passed"], 3);
}

#[test]
fn test_task_rust_integration_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path());
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 4\n---\n\n- [ ] TASK-T001: rust integration\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/integration_test.rs]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - integration works\n",
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
fn test_task_go_and_shell_adapters() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path());
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 4\n---\n\n- [ ] TASK-T001: go package\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [pkg/pkg_test.go]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - go works\n- [ ] TASK-T002: shell test\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/test_shell.sh]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - shell works\n",
    )
    .unwrap();

    for id in ["TASK-T001", "TASK-T002"] {
        let output = run_dow(dir.path(), &["test", id]);
        assert!(
            output.status.success(),
            "{} stderr: {}\nstdout: {}",
            id,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn test_unsupported_task_file_is_precondition_failure() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_project(dir.path());
    fs::write(
        doc.join("task/task_2026-07-12_1.md"),
        "---\ntitle: TASK - batch\nnums: 1\n---\n\n- [ ] TASK-T001: unsupported\n  - type: test\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: []\n      test: [tests/unknown.txt]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - unsupported is reported\n",
    )
    .unwrap();
    fs::write(dir.path().join("tests/unknown.txt"), "not a test\n").unwrap();

    let output = run_dow(dir.path(), &["test", "TASK-T001"]);
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["outcome"], "PRECONDITION_FAILED");
    assert!(fs::read_dir(doc.join("issue")).unwrap().next().is_none());
}
