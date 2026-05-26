// tests/
// ├── test_dow_status_write.rs  -- dow status 写操作集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_test_env(dir: &Path, phase: &str, mode: &str) {
    fs::create_dir_all(dir.join("dev-doc")).unwrap();
    fs::write(
        dir.join("dev-doc/STATUS.yaml"),
        format!(
            "name: test-project\nphase: {}\nmode: {}\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
            phase, mode
        ),
    ).unwrap();
    fs::write(dir.join("VERSION"), "2.8.0\n").unwrap();
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
}

fn read_field(dir: &Path, field: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--field", field])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_set_phase_valid_forward() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "SPEC", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--phase", "TASK"])
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
        .args(["status", "--phase", "DEV"])
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
        .args(["status", "--phase", "DEV"])
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
        .args(["status", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(read_field(dir.path(), "mode"), "fast");
    // fast 模式联动 phase 起点为 TASK
    assert_eq!(read_field(dir.path(), "phase"), "TASK");
}

#[test]
fn test_set_mode_audit_rejected() {
    let dir = tempfile::tempdir().unwrap();
    setup_test_env(dir.path(), "DEV", "quick");

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--mode", "audit"])
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
        .args(["status", "--exec-mode", "continuous"])
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
        .args(["status", "--name", "new-name"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let updated = read_field(dir.path(), "updated");
    // 时间戳应该被更新为今天的日期
    assert!(updated.starts_with("2026-05-26"));
    assert_ne!(updated, "2026-05-26 10:00"); // 不再是初始值
}
