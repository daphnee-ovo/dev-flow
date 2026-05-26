// tests/
// ├── test_dow_status_read.rs  -- dow status 读操作集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn dow_bin() -> &'static str {
    "./scripts/bin/dow"
}

fn setup_test_env(dir: &Path) {
    fs::create_dir_all(dir.join("dev-doc")).unwrap();
    fs::write(
        dir.join("dev-doc/STATUS.yaml"),
        "name: test-project\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    fs::write(dir.join("VERSION"), "2.8.0\n").unwrap();
    // 初始化 git 仓库以便 git tag 命令不报错
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
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
    assert_eq!(json["doc_root"], "dev-doc");
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
    assert!(stdout.contains("项目名称：test-project"));
    assert!(stdout.contains("当前阶段：DEV"));
    assert!(stdout.contains("开发模式：quick"));
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
