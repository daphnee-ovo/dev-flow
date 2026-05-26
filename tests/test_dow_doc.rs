// tests/
// ├── test_dow_doc.rs  -- dow doc 集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_env(dir: &Path) {
    let doc = dir.join("dev-doc");
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
}

#[test]
fn test_doc_task_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "--task", "-n", "5"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "task");
    assert_eq!(json["slots"], 5);

    let created = json["created"].as_str().unwrap();
    let content = fs::read_to_string(dir.path().join(created)).unwrap();
    assert!(content.contains("nums: 5"));
    assert!(content.contains("TASK-T001"));
    assert!(content.contains("TASK-T005"));
}

#[test]
fn test_doc_issue_with_source() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "--issue", "--source", "devtest", "-n", "2"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "issue");
    assert_eq!(json["slots"], 2);

    let created = json["created"].as_str().unwrap();
    assert!(created.contains("devtest"));
    let content = fs::read_to_string(dir.path().join(created)).unwrap();
    assert!(content.contains("source: devtest"));
    assert!(content.contains("ISSUE-I001"));
    assert!(content.contains("ISSUE-I002"));
}

#[test]
fn test_doc_seq_auto_increment() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    // 第一次
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "--task"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 第二次
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "--task"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let created = json["created"].as_str().unwrap();
    // 应该是 _2.md
    assert!(created.contains("_2.md"));
}

#[test]
fn test_doc_prd_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());
    fs::write(dir.path().join("dev-doc/PRD.md"), "existing").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "--prd"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("已存在"));
}
