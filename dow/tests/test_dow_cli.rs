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
