// tests/
// ├── test_dow_validate.rs  -- dow validate 集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_valid_env(dir: &Path) {
    let doc = dir.join("dev-doc");
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(doc.join("archive")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n- entry\n").unwrap();
    fs::write(dir.join(".gitignore"), "tmp/\n").unwrap();
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
}

#[test]
fn test_validate_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    setup_valid_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("validate")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code should be 0");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["auto_fixed"].as_array().unwrap().is_empty());
    assert!(json["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn test_validate_creates_missing_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("dev-doc");
    fs::create_dir_all(&doc).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("validate")
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let auto_fixed: Vec<String> = json["auto_fixed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(auto_fixed.iter().any(|s| s.contains("created_dir")));
    assert!(dir.path().join("dev-doc/task").exists());
    assert!(dir.path().join("dev-doc/issue").exists());
}

#[test]
fn test_validate_warns_invalid_phase() {
    let dir = tempfile::tempdir().unwrap();
    setup_valid_env(dir.path());
    fs::write(
        dir.path().join("dev-doc/STATUS.yaml"),
        "name: test\nphase: INVALID\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("validate")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success()); // exit 1 because warnings
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let warnings: Vec<String> = json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(warnings.iter().any(|w| w.contains("status_invalid_phase")));
}
