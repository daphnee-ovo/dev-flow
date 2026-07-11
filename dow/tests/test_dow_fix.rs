// tests/test_dow_fix.rs -- regression coverage for GitHub issues #12 and #13

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn default_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn setup_env(dir: &Path) -> PathBuf {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(branch);
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-06-23 10:00\nstarted: 2026-06-20 09:00\n",
    )
    .unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    doc
}

fn run_fix(dir: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("fix")
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "failed to parse fix output: {:?}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn test_fix_renames_fully_checked_issue_to_closed_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_other_2026-06-20_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [x] ISSUE-I001：fixed\n  - severity: P1\n  - location：a.rs:1\n  - description：done\n  - reproduce：\n  - fix：patched\n",
    )
    .unwrap();

    let result = run_fix(dir.path());
    let fixed = result["fixed"].as_array().unwrap();
    assert!(fixed.iter().any(|item| item.as_str().unwrap().contains("closed_")));
    assert!(doc.join("issue/closed_issue_other_2026-06-20_1.md").exists());
    assert!(!doc.join("issue/issue_other_2026-06-20_1.md").exists());
}

#[test]
fn test_fix_renumbers_duplicate_issue_ids_across_closed_files() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    for (date, location, title) in [
        ("2026-06-17", "a.rs:1", "old one"),
        ("2026-06-18", "b.rs:1", "old two"),
    ] {
        fs::write(
            doc.join(format!("issue/closed_issue_other_{}_1.md", date)),
            format!(
                "---\nsource: other\nnums: 1\n---\n\n- [x] ISSUE-I001：{}\n  - severity: P1\n  - location：{}\n  - description：done\n  - reproduce：\n  - fix：patched\n",
                title, location
            ),
        )
        .unwrap();
    }

    let result = run_fix(dir.path());
    let fixed = result["fixed"].as_array().unwrap();
    assert!(fixed
        .iter()
        .any(|item| item.as_str().unwrap().contains("renumbering")));

    let first = fs::read_to_string(doc.join("issue/closed_issue_other_2026-06-17_1.md"))
        .unwrap();
    let second = fs::read_to_string(doc.join("issue/closed_issue_other_2026-06-18_1.md"))
        .unwrap();
    assert!(first.contains("ISSUE-I001"));
    assert!(second.contains("ISSUE-I002"));
}
