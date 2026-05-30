// tests/
// ├── test_dow_validate.rs  -- dow validate 集成测试

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

fn setup_valid_env(dir: &Path) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n- entry\n").unwrap();
    fs::write(dir.join(".gitignore"), "tmp/\n").unwrap();
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
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();
    let branch = default_branch(dir.path());
    let doc = dir.path().join(".dev-doc").join(&branch);
    fs::create_dir_all(&doc).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();

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
    let doc_path = format!(".dev-doc/{}", branch);
    assert!(dir.path().join(&doc_path).join("task").exists());
    assert!(dir.path().join(&doc_path).join("issue").exists());
}

#[test]
fn test_validate_warns_invalid_phase() {
    let dir = tempfile::tempdir().unwrap();
    setup_valid_env(dir.path());
    let branch = default_branch(dir.path());
    fs::write(
        dir.path().join(".dev-doc").join(&branch).join("STATUS.yaml"),
        "name: test\nphase: INVALID\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("validate")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success()); // exit 1 because warnings
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let warnings = json["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|w| {
        w["message"].as_str().map(|m| m.contains("INVALID")).unwrap_or(false)
    }));
}
