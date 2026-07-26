// tests/
// ├── test_dow_issue.rs  -- dow issue subcommands integration tests

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::default_branch;

fn setup_env(dir: &Path) -> std::path::PathBuf {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-06-23 10:00\nstarted: 2026-06-20 09:00\n",
    ).unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    doc
}

// ─── create tests ────────────────────────────────────────────────────────────

#[test]
fn test_issue_create_with_flags() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--title",
            "bug in parser",
            "--severity",
            "P0",
            "--location",
            "src/parser.rs:42",
            "--desc",
            "crashes on empty input",
            "--reproduce",
            "run with empty string",
            "--source",
            "test",
            "--file",
            r#"{"modify":["src/parser.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Successful creation returns the generated issue ID.
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ISSUE-I001");

    // Verify file was created
    let issue_dir = doc.join("issue");
    let files: Vec<_> = fs::read_dir(&issue_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("issue_test_"))
        .collect();
    assert_eq!(files.len(), 1, "Expected one issue file created");

    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("ISSUE-I001"));
    assert!(content.contains("bug in parser"));
    assert!(content.contains("severity: P0"));
    assert!(content.contains("src/parser.rs:42"));
    assert!(content.contains("crashes on empty input"));
    assert!(content.contains("- [ ]"));
}

#[test]
fn test_issue_create_with_stdin_json() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    let json_input = r#"{"title":"memory leak","severity":"P1","location":"src/alloc.rs:10","desc":"grows unbounded","reproduce":"allocate in loop","source":"test","files":{"modify":["src/alloc.rs"]}}"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "create"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ISSUE-I001");
}

#[test]
fn test_issue_create_with_optional_files() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--title",
            "scoped issue",
            "--severity",
            "P1",
            "--location",
            "src/main.rs:10",
            "--desc",
            "needs a scoped fix",
            "--reproduce",
            "run the test",
            "--source",
            "test",
            "--file",
            r#"{"modify":["src/main.rs","src/lib.rs"],"create":["src/new.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let issue_file = fs::read_dir(doc.join("issue"))
        .unwrap()
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("issue_test_")
        })
        .unwrap();
    let content = fs::read_to_string(issue_file.path()).unwrap();
    assert!(content.contains("files_modify: [src/main.rs, src/lib.rs]"));
    assert!(content.contains("files_create: [src/new.rs]"));
}

#[test]
fn test_issue_create_batch_json_groups_by_source() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());
    let json_input = r#"[
        {"title":"test one","severity":"P1","location":"a.rs:1","desc":"d1","reproduce":"r1","source":"test","files":{"modify":["a.rs"]}},
        {"title":"other one","severity":"P2","location":"b.rs:2","desc":"d2","reproduce":"r2","source":"other","files":{"modify":["b.rs"]}},
        {"title":"test two","severity":"P0","location":"c.rs:3","desc":"d3","reproduce":"r3","source":"test","files":{"create":["c.rs"]}}
    ]"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "create"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["ISSUE-I001", "ISSUE-I002", "ISSUE-I003"]
    );

    let issue_files: Vec<_> = fs::read_dir(doc.join("issue")).unwrap().flatten().collect();
    assert_eq!(issue_files.len(), 2);
    let test_file = issue_files
        .iter()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("issue_test_")
        })
        .unwrap();
    let test_content = fs::read_to_string(test_file.path()).unwrap();
    assert!(test_content.contains("nums: 2"));
    assert!(test_content.contains("ISSUE-I001"));
    assert!(test_content.contains("ISSUE-I003"));
    assert!(test_content.contains("files_modify: [a.rs]"));
    assert!(test_content.contains("files_create: [c.rs]"));

    let other_file = issue_files
        .iter()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("issue_other_")
        })
        .unwrap();
    let other_content = fs::read_to_string(other_file.path()).unwrap();
    assert!(other_content.contains("nums: 1"));
    assert!(other_content.contains("ISSUE-I002"));
}

#[test]
fn test_issue_create_batch_validation_is_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());
    let json_input = r#"[
        {"title":"valid","severity":"P1","location":"a.rs:1","desc":"d","reproduce":"r","source":"test","files":{"modify":["a.rs"]}},
        {"title":"invalid","severity":"CRITICAL","location":"b.rs:1","desc":"d","reproduce":"r","source":"test","files":{"modify":["b.rs"]}}
    ]"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "create"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid severity"));
    let issue_files: Vec<_> = fs::read_dir(doc.join("issue")).unwrap().flatten().collect();
    assert!(
        issue_files.is_empty(),
        "invalid batch must not create issue files"
    );
}

#[test]
fn test_issue_create_rejects_unscoped_files() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--title",
            "unscoped",
            "--severity",
            "P1",
            "--location",
            "src/a.rs:1",
            "--desc",
            "d",
            "--reproduce",
            "r",
            "--source",
            "other",
            "--file",
            r#"{"create":[]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("files.create"));
    assert_eq!(fs::read_dir(doc.join("issue")).unwrap().count(), 0);
}

#[test]
fn test_issue_create_rejects_mixed_cli_and_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());
    let json_input = r#"{"title":"stdin issue","severity":"P1","location":"src/a.rs:1","desc":"d","reproduce":"r","source":"other","files":{"modify":["src/a.rs"]}}"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "create", "--title", "cli issue"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot combine"));
}

#[test]
fn test_issue_update_nested_files_incremental() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());
    let issue_path = doc.join("issue/issue_other_2026-06-25_1.md");
    fs::write(
        &issue_path,
        "---\nsource: other\nnums: 1\n---\n\n- [ ] ISSUE-I001：Update scope\n  - severity: P1\n  - location：src/old.rs:1\n  - description：d\n  - reproduce：r\n  - files_modify: [old.rs]\n  - fix：\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "update",
            "ISSUE-I001",
            "--file",
            r#"{"modify":["+new.rs","-old.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = fs::read_to_string(issue_path).unwrap();
    assert!(content.contains("files_modify: [new.rs]"));
    assert!(!content.contains("files_modify: [old.rs]"));
}

#[test]
fn test_issue_update_rejects_task_test_scope() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());
    let issue_path = doc.join("issue/issue_other_2026-06-25_1.md");
    fs::write(
        &issue_path,
        "---\nsource: other\nnums: 1\n---\n\n- [ ] ISSUE-I001：Issue\n  - severity: P1\n  - location：src/a.rs:1\n  - description：d\n  - reproduce：r\n  - files_modify: [src/a.rs]\n  - fix：\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "update",
            "ISSUE-I001",
            "--file",
            r#"{"test":["tests/issue.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown field"));
}

#[test]
fn test_issue_create_rejects_fix_in_json() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    // stdin JSON carrying a `fix` field must be rejected transparently,
    // not silently dropped.
    let json_input = r#"{"title":"x","severity":"P1","location":"src/a.rs:1","desc":"d","reproduce":"r","source":"test","files":{"modify":["src/a.rs"]},"fix":"already fixed"}"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "create"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        !output.status.success(),
        "create should fail when `fix` is provided"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("update") && stderr.contains("--fix"),
        "stderr should point to `dow issue update <id> --fix`, got: {}",
        stderr
    );
    // Must not mislead users toward the non-existent `close --fix`.
    assert!(
        !stderr.contains("close --fix"),
        "stderr must not suggest `close --fix` (close has no --fix flag), got: {}",
        stderr
    );
}

#[test]
fn test_issue_create_invalid_severity() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--title",
            "test",
            "--severity",
            "CRITICAL",
            "--location",
            "a.rs:1",
            "--desc",
            "test desc",
            "--reproduce",
            "steps",
            "--source",
            "other",
            "--file",
            r#"{"modify":["a.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid severity"), "stderr: {}", stderr);
}

#[test]
fn test_issue_create_missing_title() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--severity",
            "P1",
            "--file",
            r#"{"modify":["a.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--title is required"), "stderr: {}", stderr);
}

#[test]
fn test_issue_create_auto_increments_id() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // Pre-populate an existing issue
    fs::write(
        doc.join("issue/issue_other_2026-06-20_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [ ] ISSUE-I001：existing issue\n  - severity: P1\n  - location：a.rs:1\n  - description：test\n  - reproduce：\n  - fix：\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "issue",
            "create",
            "--title",
            "second issue",
            "--severity",
            "P2",
            "--location",
            "b.rs:5",
            "--desc",
            "another bug",
            "--reproduce",
            "steps here",
            "--source",
            "test",
            "--file",
            r#"{"modify":["b.rs"]}"#,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Find the new file
    let issue_dir = doc.join("issue");
    let files: Vec<_> = fs::read_dir(&issue_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("issue_test_")
        })
        .collect();
    assert_eq!(files.len(), 1);

    let content = fs::read_to_string(files[0].path()).unwrap();
    assert!(
        content.contains("ISSUE-I002"),
        "should get ID 002, got: {}",
        content
    );
}

// ─── show tests ──────────────────────────────────────────────────────────────

#[test]
fn test_issue_show() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：parser crash\n  - severity: P0\n  - location：src/parser.rs:42\n  - description：crashes on empty\n  - reproduce：run with empty file\n  - files_modify: [src/parser.rs]\n  - fix：\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "show", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["id"], "ISSUE-I001");
    assert_eq!(json["title"], "parser crash");
    assert_eq!(json["severity"], "P0");
    assert_eq!(json["location"], "src/parser.rs:42");
    assert_eq!(json["files"]["modify"][0], "src/parser.rs");
    assert_eq!(json["status"], "open");
}

#[test]
fn test_issue_show_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "show", "ISSUE-I999"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

#[test]
fn test_issue_show_closed() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I001：fixed bug\n  - severity: P1\n  - location：a.rs:1\n  - description：was broken\n  - reproduce：\n  - fix：patched\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "show", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "closed");
    assert_eq!(json["fix"], "patched");
}

// ─── close tests ─────────────────────────────────────────────────────────────

#[test]
fn test_issue_close() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：bug to fix\n  - severity: P0\n  - location：x.rs:1\n  - description：broken\n  - reproduce：\n  - fix：patched the null check\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "close", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "should be silent on success");

    // File should be renamed to closed_
    assert!(doc.join("issue/closed_issue_test_2026-06-20_1.md").exists());
    assert!(!doc.join("issue/issue_test_2026-06-20_1.md").exists());

    // Content should have [x]
    let content = fs::read_to_string(doc.join("issue/closed_issue_test_2026-06-20_1.md")).unwrap();
    assert!(content.contains("- [x]"));
    assert!(!content.contains("- [ ]"));
}

#[test]
fn test_issue_close_already_closed() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I001：already done\n  - severity: P1\n  - location：a.rs:1\n  - description：done\n  - reproduce：\n  - fix：fixed\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "close", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already closed"), "stderr: {}", stderr);
}

#[test]
fn test_issue_close_multi_item_file_partial() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // File with 2 issues, close only one
    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 2\n---\n\n- [ ] ISSUE-I001：first bug\n  - severity: P1\n  - location：a.rs:1\n  - description：bug1\n  - reproduce：\n  - fix：fixed null pointer\n- [ ] ISSUE-I002：second bug\n  - severity: P2\n  - location：b.rs:2\n  - description：bug2\n  - reproduce：\n  - fix：\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "close", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File should NOT be renamed (still has open issues)
    assert!(doc.join("issue/issue_test_2026-06-20_1.md").exists());
    assert!(!doc.join("issue/closed_issue_test_2026-06-20_1.md").exists());

    let content = fs::read_to_string(doc.join("issue/issue_test_2026-06-20_1.md")).unwrap();
    assert!(
        content.contains("- [x] ISSUE-I001"),
        "I001 should be checked"
    );
    assert!(
        content.contains("- [ ] ISSUE-I002"),
        "I002 should remain unchecked"
    );
}

// ─── reopen tests ────────────────────────────────────────────────────────────

#[test]
fn test_issue_reopen_preview() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I001：closed bug\n  - severity: P1\n  - location：a.rs:1\n  - description：was broken\n  - reproduce：\n  - fix：fixed it\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "reopen", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["id"], "ISSUE-I001");
    assert!(json["confirm_token"].as_str().unwrap().starts_with("IRO-"));
    assert!(json["command"].as_str().unwrap().contains("--confirm"));
}

#[test]
fn test_issue_reopen_with_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I001：closed bug\n  - severity: P1\n  - location：a.rs:1\n  - description：was broken\n  - reproduce：\n  - fix：fixed it\n",
    ).unwrap();

    // First get the token
    let preview_output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "reopen", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&preview_output.stdout).unwrap();
    let token = json["confirm_token"].as_str().unwrap();

    // Now reopen with confirm
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "reopen", "ISSUE-I001", "--confirm", token])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "should be silent on success");

    // File should be renamed back (no closed_ prefix)
    assert!(doc.join("issue/issue_test_2026-06-20_1.md").exists());
    assert!(!doc.join("issue/closed_issue_test_2026-06-20_1.md").exists());

    // Content should have [ ] again
    let content = fs::read_to_string(doc.join("issue/issue_test_2026-06-20_1.md")).unwrap();
    assert!(content.contains("- [ ]"));
    assert!(!content.contains("- [x]"));
}

#[test]
fn test_issue_reopen_wrong_token() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I001：closed bug\n  - severity: P1\n  - location：a.rs:1\n  - description：was broken\n  - reproduce：\n  - fix：fixed it\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "reopen", "ISSUE-I001", "--confirm", "IRO-000000"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Token mismatch") || stderr.contains("mismatch"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_issue_reopen_not_closed() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：open bug\n  - severity: P1\n  - location：a.rs:1\n  - description：still open\n  - reproduce：\n  - fix：\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "reopen", "ISSUE-I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not closed"), "stderr: {}", stderr);
}

// ─── schema tests ────────────────────────────────────────────────────────────

#[test]
fn test_issue_schema() {
    let dir = tempfile::tempdir().unwrap();
    // schema doesn't require .dev-doc, but we init git for doc_root resolution
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "schema"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["fields"].is_array());
    let fields = json["fields"].as_array().unwrap();
    assert!(fields.len() >= 5);

    // Check that severity field has valid_values
    let severity_field = fields.iter().find(|f| f["name"] == "severity").unwrap();
    assert!(severity_field["valid_values"].as_array().unwrap().len() == 3);

    let files = fields.iter().find(|f| f["name"] == "files").unwrap();
    assert_eq!(files["required"], true);
    assert_eq!(files["type"], "object");
    assert_eq!(
        files["at_least_one"],
        serde_json::json!(["create", "modify"])
    );
    assert!(files["properties"]["create"].is_object());
    assert!(files["properties"]["modify"].is_object());

    assert!(json["file_format"].as_str().unwrap().contains("issue_"));
    assert!(json["id_format"].as_str().unwrap().contains("ISSUE-I"));
}

// ─── list tests ──────────────────────────────────────────────────────────────

#[test]
fn test_issue_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _doc = setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["total"], 0);
}

#[test]
fn test_issue_list_shows_open_only() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：open bug\n  - severity: P0\n  - location：a.rs:1\n  - description：test\n  - reproduce：\n  - fix：\n",
    ).unwrap();
    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_2.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I002：closed bug\n  - severity: P1\n  - location：b.rs:1\n  - description：done\n  - reproduce：\n  - fix：fixed\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["total"], 1);
}

#[test]
fn test_issue_list_all() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    fs::write(
        doc.join("issue/issue_test_2026-06-20_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：open bug\n  - severity: P0\n  - location：a.rs:1\n  - description：test\n  - reproduce：\n  - fix：\n",
    ).unwrap();
    fs::write(
        doc.join("issue/closed_issue_test_2026-06-20_2.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [x] ISSUE-I002：closed bug\n  - severity: P1\n  - location：b.rs:1\n  - description：done\n  - reproduce：\n  - fix：fixed\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "list", "--all"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let open = json["open"].as_array().unwrap();
    assert_eq!(open.len(), 2, "should show both open and closed files");
}

// ─── Short ID Tests ─────────────────────────────────────────────────────────

#[test]
fn test_issue_show_accepts_short_id() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    let issue_dir = doc.join("issue");
    fs::write(
        issue_dir.join("issue_test_2026-07-22_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：Short ID bug\n  - severity: P1\n  - location：src/lib.rs:10\n  - description：test short id\n  - reproduce：use I1\n",
    )
    .unwrap();

    // I001 (short with padding)
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "show", "I001"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "I001 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["id"], "ISSUE-I001");

    // I1 (short without padding)
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "show", "I1"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "I1 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["id"], "ISSUE-I001");
}

#[test]
fn test_issue_close_accepts_short_id() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    let issue_dir = doc.join("issue");
    fs::write(
        issue_dir.join("issue_test_2026-07-22_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：Close short\n  - severity: P1\n  - location：a.rs:1\n  - description：close with short\n  - reproduce：use I1\n  - fix：patched\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["issue", "close", "I1"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "I1 close failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
