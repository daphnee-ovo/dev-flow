// tests/test_dow_cli.rs — CLI command rename and compatibility routing

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_doctor_fixture(dir: &Path) {
    common::git_init_with_commit(dir);
    common::setup_dev_doc(dir, "DEV", "fast");

    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::write(dir.join(".gitignore"), "tmp/\n.dev-doc/**/claim.lock\n").unwrap();

    let branch = common::default_branch(dir);
    let issue_dir = dir.join(".dev-doc").join(branch).join("issue");
    fs::write(
        issue_dir.join("issue_other_2026-05-26_1.md"),
        "- [ ] ISSUE-I001: malformed issue\n  - severity: P1\n  - location: src/main.rs:1\n  - description: missing frontmatter\n  - reproduce: run test\n  - fix:\n",
    )
    .unwrap();
}

fn run_dow(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn setup_claim_fixture(dir: &Path) {
    common::git_init_with_commit(dir);
    common::setup_dev_doc(dir, "DEV", "fast");
    let branch = common::default_branch(dir);
    let task_dir = dir.join(".dev-doc").join(branch).join("task");
    fs::write(
        task_dir.join("task_2026-08-02_1.md"),
        "- [ ] TASK-T001: claim timeout test\n  - priority: P1\n",
    )
    .unwrap();
    fs::write(dir.join(".gitignore"), ".dev-doc/**/claim.lock\n").unwrap();
}

fn setup_issue_claim_fixture(dir: &Path) {
    common::git_init_with_commit(dir);
    common::setup_dev_doc(dir, "DEV", "fast");
    let branch = common::default_branch(dir);
    let issue_dir = dir.join(".dev-doc").join(branch).join("issue");
    fs::write(
        issue_dir.join("issue_test_2026-08-02_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001：claim output test\n  - severity: P1\n  - location：src/main.rs:1\n  - description：claim output should include details\n  - reproduce：run dow claim\n  - files_modify: [src/main.rs]\n  - fix：\n",
    )
    .unwrap();
    fs::write(dir.join(".gitignore"), ".dev-doc/**/claim.lock\n").unwrap();
}

#[test]
fn claim_uses_ten_minute_default_and_allows_thirty_minute_maximum() {
    let dir = tempfile::tempdir().unwrap();
    setup_claim_fixture(dir.path());

    let default_claim = run_dow(dir.path(), &["claim", "T001"]);
    assert!(default_claim.status.success());
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(
            dir.path()
                .join(".dev-doc")
                .join(common::default_branch(dir.path()))
                .join("claim.lock"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(lock["ttl"], 600);

    let maximum = run_dow(dir.path(), &["claim", "T001", "--timeout", "1800"]);
    assert!(maximum.status.success());
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(
            dir.path()
                .join(".dev-doc")
                .join(common::default_branch(dir.path()))
                .join("claim.lock"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(lock["ttl"], 1800);
}

#[test]
fn claim_rejects_timeout_above_thirty_minutes() {
    let dir = tempfile::tempdir().unwrap();
    setup_claim_fixture(dir.path());

    let output = run_dow(dir.path(), &["claim", "T001", "--timeout", "1801"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("<= 1800 seconds"));
}

#[test]
fn claim_prints_task_and_issue_details_on_success() {
    let task_dir = tempfile::tempdir().unwrap();
    setup_claim_fixture(task_dir.path());

    let task_claim = run_dow(task_dir.path(), &["claim", "T001"]);
    assert!(
        task_claim.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task_claim.stderr)
    );
    let task_claim_json: serde_json::Value = serde_json::from_slice(&task_claim.stdout).unwrap();
    let task_show = run_dow(task_dir.path(), &["task", "show", "T001"]);
    let task_show_json: serde_json::Value = serde_json::from_slice(&task_show.stdout).unwrap();
    assert_eq!(task_claim_json, task_show_json);

    let issue_dir = tempfile::tempdir().unwrap();
    setup_issue_claim_fixture(issue_dir.path());

    let issue_claim = run_dow(issue_dir.path(), &["claim", "I001"]);
    assert!(
        issue_claim.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&issue_claim.stderr)
    );
    let issue_claim_json: serde_json::Value = serde_json::from_slice(&issue_claim.stdout).unwrap();
    let issue_show = run_dow(issue_dir.path(), &["issue", "show", "I001"]);
    let issue_show_json: serde_json::Value = serde_json::from_slice(&issue_show.stdout).unwrap();
    assert_eq!(issue_claim_json, issue_show_json);
}

#[test]
fn doctor_replaces_lint_as_public_command() {
    let help = run_dow(Path::new("."), &["doctor", "--help"]);
    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("Usage: dow doctor"));
    assert!(stdout.contains("--fix"));

    let legacy = run_dow(Path::new("."), &["lint"]);
    assert!(!legacy.status.success());
    assert!(String::from_utf8_lossy(&legacy.stderr).contains("lint"));
}

#[test]
fn fix_alias_matches_doctor_fix() {
    let doctor_dir = tempfile::tempdir().unwrap();
    let fix_dir = tempfile::tempdir().unwrap();
    setup_doctor_fixture(doctor_dir.path());
    setup_doctor_fixture(fix_dir.path());

    let doctor = run_dow(doctor_dir.path(), &["doctor", "--fix"]);
    let fix = run_dow(fix_dir.path(), &["fix"]);

    assert_eq!(doctor.status.code(), fix.status.code());
    assert_eq!(doctor.status.success(), fix.status.success());

    let doctor_json: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    let fix_json: serde_json::Value = serde_json::from_slice(&fix.stdout).unwrap();
    assert_eq!(doctor_json, fix_json);

    let doctor_branch = common::default_branch(doctor_dir.path());
    let fix_branch = common::default_branch(fix_dir.path());
    for (dir, branch) in [
        (doctor_dir.path(), doctor_branch),
        (fix_dir.path(), fix_branch),
    ] {
        let issue = dir
            .join(".dev-doc")
            .join(branch)
            .join("issue/issue_other_2026-05-26_1.md");
        assert!(issue.exists());
        assert!(fs::read_to_string(issue)
            .unwrap()
            .starts_with("---\nsource: other\n"));
    }
}
