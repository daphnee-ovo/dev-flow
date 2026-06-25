// tests/
// ├── test_dow_status_read.rs  -- dow status 读操作集成测试

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::default_branch;

fn setup_test_env(dir: &Path) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(&doc).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test-project\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    fs::write(dir.join("VERSION"), format!("({})2.8.0\n", branch)).unwrap();
}

#[test]
fn test_status_json_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("status")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["name"], "test-project");
    assert_eq!(json["phase"], "DEV");
    assert_eq!(json["mode"], "quick");
    assert_eq!(json["version"], "2.8.0");
    assert_eq!(json["version_tag"], "no-tag");
    // doc_root 应为 .dev-doc/<branch> 格式
    let doc_root = json["doc_root"].as_str().unwrap();
    assert!(doc_root.starts_with(".dev-doc/"), "doc_root should be branch-specific: {}", doc_root);
}

#[test]
fn test_status_field() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--field", "phase"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "DEV");
}

#[test]
fn test_status_human_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "-H"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Project Name: test-project"));
    assert!(stdout.contains("Current Phase: DEV"));
    assert!(stdout.contains("Development Mode: quick"));
}

#[test]
fn test_status_missing_status_yaml() {
    let dir = tempfile::tempdir().unwrap();
    // 不创建 STATUS.yaml

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("status")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("STATUS.yaml"));
}
