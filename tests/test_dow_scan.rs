// tests/
// ├── test_dow_scan.rs  -- dow scan 集成测试

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_project(dir: &Path) {
    fs::write(dir.join("VERSION"), "1.0.0\n").unwrap();
    fs::write(dir.join("README.md"), "# Test Project\nA test.\n").unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    // Cargo.toml 使得检测到 rust
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"test-proj\"\nversion = \"0.1.0\"\n",
    ).unwrap();
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init", "--allow-empty"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn test_scan_detects_rust() {
    let dir = tempfile::tempdir().unwrap();
    setup_project(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("scan")
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], "test-proj");
    let stack: Vec<String> = json["tech_stack"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(stack.contains(&"rust".to_string()));
}

#[test]
fn test_scan_human_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_project(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["scan", "-H"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PROJECT SCAN"));
    assert!(stdout.contains("test-proj"));
}
