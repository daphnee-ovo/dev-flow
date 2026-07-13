// tests/
// ├── test_dow_status_write.rs  -- dow status 写操作集成测试

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::{default_branch, read_status_field as read_field};

fn setup_test_env(dir: &Path, phase: &str, mode: &str) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    common::setup_dev_doc(dir, phase, mode);
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::write(
        doc.join("task/task_2026-05-26_1.md"),
        "---\ntitle: TASK - test\nnums: 1\n---\n\n- [ ] TASK-T001: test\n  - priority: P1\n  - complexity: S\n  - done_when:\n      - passes\n",
    ).unwrap();
    fs::write(dir.join("VERSION"), format!("({})2.8.0\n", branch)).unwrap();
}

#[test]
fn test_set_phase_valid_forward() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "SPEC", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--phase", "TASK"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(read_field(dir.path(), "phase"), "TASK");
}

#[test]
fn test_set_phase_invalid_skip() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "SPEC", "quick");

    // quick 模式下 SPEC 不能直接跳到 DEV（必须经过 TASK）
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--phase", "DEV"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(read_field(dir.path(), "phase"), "SPEC"); // 未改变
}

#[test]
fn test_set_phase_test_to_dev_always_allowed() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "TEST", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--phase", "DEV"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(read_field(dir.path(), "phase"), "DEV");
}

#[test]
fn test_set_mode_valid() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "SPEC", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(read_field(dir.path(), "mode"), "fast");
    // mode 切换不再联动 phase（phase 保持不变）
    assert_eq!(read_field(dir.path(), "phase"), "SPEC");
}

#[test]
fn test_set_mode_audit_rejected() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "DEV", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--mode", "audit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(read_field(dir.path(), "mode"), "quick"); // 未改变
}

#[test]
fn test_set_exec_mode() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "DEV", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--exec-mode", "continuous"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(read_field(dir.path(), "exec_mode"), "continuous");
}

#[test]
fn test_write_updates_timestamp() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "DEV", "quick");

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "set", "--name", "new-name"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let updated = read_field(dir.path(), "updated");
    // 时间戳应该被更新（不再是初始值）
    assert_ne!(updated, "2026-05-26 10:00");
    // 应该是合法的日期时间格式
    assert!(
        updated.len() >= 10,
        "updated should be datetime: {}",
        updated
    );
}
