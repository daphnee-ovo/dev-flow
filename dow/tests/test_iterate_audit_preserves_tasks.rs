// tests/
// ├── test_iterate_audit_preserves_tasks.rs
//    -- audit mode iterate preserves incomplete task_* files

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_audit_iterate_env(dir: &Path) {
    common::git_init_with_commit(dir);

    let branch = common::default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();

    // audit/fast mode — skips task completion check
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: audit/fast\nupdated: 2026-07-23 10:00\nstarted: 2026-07-23 09:00\n",
    )
    .unwrap();

    // One completed task file
    fs::write(
        doc.join("task/done_task_2026-07-23_1.md"),
        "---\ntitle: TASK - batch 2026-07-23\nnums: 1\n---\n\n- [x] TASK-T001: completed task\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - ok\n",
    )
    .unwrap();

    // One incomplete task file — should be preserved in audit mode
    fs::write(
        doc.join("task/task_2026-07-23_2.md"),
        "---\ntitle: TASK - batch 2026-07-23 second\nnums: 1\n---\n\n- [ ] TASK-T002: incomplete task\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - pending\n",
    )
    .unwrap();

    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n- audit iteration\n").unwrap();

    // VERSION
    fs::write(dir.join("VERSION"), format!("({})0.1.0\n", branch)).unwrap();

    fs::write(dir.join(".gitignore"), "tmp/\n").unwrap();

    // Commit everything
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "setup"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn run_dow(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn audit_iterate_preserves_incomplete_task_files() {
    let dir = tempfile::tempdir().unwrap();
    setup_audit_iterate_env(dir.path());

    // Preview to get the confirmation token
    let preview = run_dow(
        dir.path(),
        &["iterate", "--topic", "audit-test", "--type", "feat"],
    );
    assert!(preview.status.success(), "preview failed: {}", String::from_utf8_lossy(&preview.stderr));

    let stdout = String::from_utf8_lossy(&preview.stdout);
    // save_changelog may print a line before JSON; extract the JSON object
    let json_start = stdout.find('{').expect("no JSON in stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..])
        .unwrap_or_else(|e| panic!("JSON parse failed: {}\nstdout: {}", e, stdout));
    let token = json["token"].as_str().unwrap().to_string();

    // Confirm execution
    let confirm = run_dow(
        dir.path(),
        &["iterate", "--topic", "audit-test", "--type", "feat", "--confirm", &token],
    );
    assert!(confirm.status.success(), "confirm failed: {}", String::from_utf8_lossy(&confirm.stderr));

    let branch = common::default_branch(dir.path());
    let doc = dir.path().join(".dev-doc").join(&branch);

    // Completed task file should be deleted (archived)
    assert!(
        !doc.join("task/done_task_2026-07-23_1.md").exists(),
        "done_task file should have been archived and deleted"
    );

    // Incomplete task file should still exist
    assert!(
        doc.join("task/task_2026-07-23_2.md").exists(),
        "incomplete task_* file should be preserved in audit mode"
    );
}

#[test]
fn normal_mode_iterate_archives_all_task_files() {
    let dir = tempfile::tempdir().unwrap();
    common::git_init_with_commit(dir.path());

    let branch = common::default_branch(dir.path());
    let doc = dir.path().join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();

    // Normal fast mode — requires all tasks complete
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: fast\nupdated: 2026-07-23 10:00\nstarted: 2026-07-23 09:00\n",
    )
    .unwrap();

    // All tasks marked done (in a done_task file)
    fs::write(
        doc.join("task/done_task_2026-07-23_1.md"),
        "---\ntitle: TASK - batch 2026-07-23\nnums: 1\n---\n\n- [x] TASK-T001: done\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - ok\n",
    )
    .unwrap();

    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n- normal iteration\n").unwrap();
    fs::write(dir.path().join("VERSION"), format!("({})0.2.0\n", branch)).unwrap();
    fs::write(dir.path().join(".gitignore"), "tmp/\n").unwrap();

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "setup"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Preview
    let preview = run_dow(
        dir.path(),
        &["iterate", "--topic", "normal-test", "--type", "feat"],
    );
    assert!(preview.status.success());

    let stdout = String::from_utf8_lossy(&preview.stdout);
    let json_start = stdout.find('{').expect("no JSON in stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    // Confirm
    let confirm = run_dow(
        dir.path(),
        &["iterate", "--topic", "normal-test", "--type", "feat", "--confirm", &token],
    );
    assert!(confirm.status.success(), "confirm failed: {}", String::from_utf8_lossy(&confirm.stderr));

    // All task files should be archived and deleted in normal mode
    assert!(
        !doc.join("task/done_task_2026-07-23_1.md").exists(),
        "done_task file should have been archived and deleted in normal mode"
    );
}
