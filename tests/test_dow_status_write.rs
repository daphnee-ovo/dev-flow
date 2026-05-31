// tests/
// ├── test_dow_status_write.rs  -- dow status 写操作集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn default_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn setup_test_env(dir: &Path, phase: &str, mode: &str) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        format!(
            "name: test-project\nphase: {}\nmode: {}\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
            phase, mode
        ),
    ).unwrap();
    fs::write(
        doc.join("task/task_2026-05-26_1.md"),
        "---\ntitle: TASK - test\nnums: 1\n---\n\n- [ ] TASK-T001: test\n  - priority: P1\n  - complexity: S\n  - done_when:\n      - passes\n",
    ).unwrap();
    fs::write(dir.join("VERSION"), format!("({})2.8.0\n", branch)).unwrap();
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
    // mode 切换不再联动 phase（phase 保持不变）
    assert_eq!(read_field(dir.path(), "phase"), "SPEC");
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
    // 时间戳应该被更新（不再是初始值）
    assert_ne!(updated, "2026-05-26 10:00");
    // 应该是合法的日期时间格式
    assert!(updated.len() >= 10, "updated should be datetime: {}", updated);
}
