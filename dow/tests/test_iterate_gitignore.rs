// tests/
// ├── test_iterate_gitignore.rs  -- iterate 在 .dev-doc/ 被 gitignore 时跳过 commit

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_iterate_env(dir: &Path, gitignore_dev_doc: bool) {
    common::git_init_with_commit(dir);

    let branch = common::default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: fast\nupdated: 2026-07-14 10:00\nstarted: 2026-07-14 09:00\n",
    )
    .unwrap();
    // 所有 task 标记完成以通过 iterate 检查
    fs::write(
        doc.join("task/done_task_2026-07-14_1.md"),
        "---\ntitle: TASK - batch 2026-07-14\nnums: 1\n---\n\n- [x] TASK-T001: done\n  - type: feat\n  - priority: P1\n  - refs:\n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - ok\n",
    )
    .unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n- test entry\n").unwrap();

    // VERSION — format: (branch)version
    fs::write(dir.join("VERSION"), format!("({})0.1.0\n", branch)).unwrap();

    // gitignore
    if gitignore_dev_doc {
        fs::write(dir.join(".gitignore"), ".dev-doc/\n").unwrap();
    } else {
        fs::write(dir.join(".gitignore"), "tmp/\n").unwrap();
    }

    // commit everything tracked
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

    // 创建 archive.db 让 iterate 有东西可以提交
    fs::write(dir.join(".dev-doc/archive.db"), "fake").unwrap();
}

fn run_dow(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn iterate_preview_excludes_devdoc_when_gitignored() {
    let dir = tempfile::tempdir().unwrap();
    setup_iterate_env(dir.path(), true);

    let output = run_dow(
        dir.path(),
        &["iterate", "--topic", "test", "--type", "feat"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // preview 输出不应包含 .dev-doc/ 路径在 commit_files 中
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
    let commit_files = json["commit_files"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for f in &commit_files {
        assert!(
            !f.starts_with(".dev-doc/"),
            "commit_files should not contain .dev-doc/ paths when gitignored, found: {}",
            f
        );
    }
}

#[test]
fn iterate_preview_includes_devdoc_when_not_gitignored() {
    let dir = tempfile::tempdir().unwrap();
    setup_iterate_env(dir.path(), false);

    // 标记 .dev-doc 路径有改动（git add 使其 tracked）
    Command::new("git")
        .args(["add", ".dev-doc/"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = run_dow(
        dir.path(),
        &["iterate", "--topic", "test", "--type", "feat"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "iterate preview failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {}; stdout={}", e, stdout));
    let archive_db = json["archive_db"].as_str().unwrap_or("");
    assert!(
        archive_db.contains(".dev-doc/archive.db"),
        "archive_db should reference .dev-doc/archive.db, got: {}",
        archive_db
    );
}
