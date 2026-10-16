mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> tempfile::TempDir {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp/test_target_project");
    fs::create_dir_all(&project_root).unwrap();
    let dir = tempfile::Builder::new()
        .prefix("dow-verbose-")
        .tempdir_in(project_root)
        .unwrap();
    common::git_init_with_commit(dir.path());
    common::setup_dev_doc(dir.path(), "DEV", "fast");
    dir
}

fn task_dir(dir: &Path) -> PathBuf {
    dir.join(".dev-doc")
        .join(common::default_branch(dir))
        .join("task")
}

fn write_task(dir: &Path, test_files: &[&str]) {
    fs::write(
        task_dir(dir).join("task_2026-10-17_1.md"),
        format!(
            "---\ntitle: TASK - verbose\nnums: 1\n---\n\n{}",
            task_entry("TASK-T001", "Verbose task", test_files)
        ),
    )
    .unwrap();
}

fn task_entry(id: &str, title: &str, test_files: &[&str]) -> String {
    let tests = test_files
        .iter()
        .map(|file| format!("\"{}\"", file))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "- [ ] {}: {}\n  - type: feat\n  - priority: P1\n  - refs: user-request\n  - files:\n      create: []\n      modify: []\n      test: [{}]\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - command output is visible\n",
        id, title, tests
    )
}

fn run_dow(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn task_done_verbose_streams_success_output_and_completion_status() {
    let dir = fixture();
    write_task(dir.path(), &["tests/pass.sh"]);
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/pass.sh"),
        "#!/bin/sh\nprintf 'visible test output\\n'\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["task", "done", "T001", "--verbose"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let test_name = "Shell tests/pass.sh";
    assert!(output.status.success(), "stdout: {}", stdout);
    assert!(stdout.contains(&format!("[dev-flow] TASK-T001 {}\n", test_name)));
    assert!(stdout.lines().any(|line| line == "visible test output"));
    assert!(stdout.lines().any(|line| line == "TASK-T001 Shell tests/pass.sh PASS"));
    assert!(stdout.contains("---\n"), "stdout: {}", stdout);
    assert!(stdout
        .lines()
        .any(|line| line == "[dev-flow] TASK-T001 completed"));
    assert!(!stdout.contains("Command:"));
    assert!(!stdout.contains("[dev-flow] Shell tests/pass.sh stdout:"));
    assert!(
        fs::read_to_string(task_dir(dir.path()).join("done_task_2026-10-17_1.md"))
            .unwrap()
            .contains("- [x] TASK-T001:")
    );
}

#[test]
fn task_done_verbose_streams_failure_and_reports_task_not_completed() {
    let dir = fixture();
    write_task(dir.path(), &["tests/fail.sh"]);
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/fail.sh"),
        "#!/bin/sh\nprintf 'stdout failure detail\\n'\nprintf 'stderr failure detail\\n' >&2\nexit 1\n",
    )
    .unwrap();

    let output = run_dow(dir.path(), &["task", "done", "TASK-T001", "-v"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(stdout.lines().any(|line| line == "stdout failure detail"));
    assert!(stdout.lines().any(|line| line == "stderr failure detail"));
    assert!(stdout.contains("---\n"), "stdout: {}", stdout);
    assert!(stdout.contains("TASK-T001 Shell tests/fail.sh FAIL"));
    assert!(stdout
        .lines()
        .any(|line| line == "[dev-flow] TASK-T001 failed"));
    assert!(
        fs::read_to_string(task_dir(dir.path()).join("task_2026-10-17_1.md"))
            .unwrap()
            .contains("- [ ] TASK-T001:")
    );
}

#[test]
fn task_done_verbose_without_tests_only_reports_completion() {
    let dir = fixture();
    write_task(dir.path(), &[]);

    let output = run_dow(dir.path(), &["task", "done", "T001", "-v"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout.trim(), "[dev-flow] TASK-T001 completed");
}

#[test]
fn task_done_verbose_aggregates_multiple_task_ids() {
    let dir = fixture();
    let task_content = format!(
        "---\ntitle: TASK - verbose batch\nnums: 2\n---\n\n{}\n{}",
        task_entry("TASK-T001", "First task", &["tests/pass.sh"]),
        task_entry("TASK-T002", "Second task", &["tests/fail.sh"])
    );
    fs::write(
        task_dir(dir.path()).join("task_2026-10-17_1.md"),
        task_content,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/pass.sh"),
        "#!/bin/sh\nprintf 'first output\\n'\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/fail.sh"),
        "#!/bin/sh\nprintf 'second output\\n'\nexit 1\n",
    )
    .unwrap();

    let output = run_dow(
        dir.path(),
        &["task", "done", "TASK-T001", "TASK-T002", "-v"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(stdout.contains("[dev-flow] TASK-T001 completed, TASK-T002 failed"));
    assert!(stdout.contains("TASK-T001 Shell tests/pass.sh PASS"));
    assert!(stdout.contains("TASK-T002 Shell tests/fail.sh FAIL"));
}

#[test]
fn verbose_aliases_show_task_details_and_claim_remaining_time() {
    let dir = fixture();
    write_task(dir.path(), &[]);

    let task_show = run_dow(dir.path(), &["task", "show", "T001", "-v"]);
    assert!(task_show.status.success());
    let task_json: serde_json::Value = serde_json::from_slice(&task_show.stdout).unwrap();
    assert_eq!(task_json["files"]["test"], serde_json::json!([]));

    let claim = run_dow(dir.path(), &["claim", "T001", "-v"]);
    let stdout = String::from_utf8_lossy(&claim.stdout);
    assert!(
        claim.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&claim.stderr)
    );
    assert!(stdout.contains("Verbose task"), "stdout: {}", stdout);
    assert!(stdout.contains("TASK-T001"), "stdout: {}", stdout);
    assert!(stdout.contains("seconds remaining"), "stdout: {}", stdout);
}

#[test]
fn claim_without_verbose_preserves_json_output() {
    let dir = fixture();
    write_task(dir.path(), &[]);

    let claim = run_dow(dir.path(), &["claim", "T001"]);
    assert!(claim.status.success());
    let value: serde_json::Value = serde_json::from_slice(&claim.stdout).unwrap();
    assert_eq!(value["id"], "TASK-T001");
    assert!(value.get("remaining_secs").is_none());
}

#[test]
fn verbose_claim_shows_issue_details_and_remaining_time() {
    let dir = fixture();
    let issue_dir = dir
        .path()
        .join(".dev-doc")
        .join(common::default_branch(dir.path()))
        .join("issue");
    fs::write(
        issue_dir.join("issue_verbose_2026-10-17_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001: Verbose issue\n  - severity: P1\n  - location: src/main.rs:1\n  - description: claim output details\n  - reproduce: run dow claim\n  - files_modify: [src/main.rs]\n  - fix:\n",
    )
    .unwrap();

    let claim = run_dow(dir.path(), &["claim", "I001", "--verbose"]);
    let stdout = String::from_utf8_lossy(&claim.stdout);
    assert!(
        claim.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&claim.stderr)
    );
    assert!(stdout.contains("Verbose issue"), "stdout: {}", stdout);
    assert!(stdout.contains("ISSUE-I001"), "stdout: {}", stdout);
    assert!(stdout.contains("seconds remaining"), "stdout: {}", stdout);
}
