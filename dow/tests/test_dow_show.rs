mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn isolated_test_project() -> tempfile::TempDir {
    let target_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tmp/test_target_project");
    fs::create_dir_all(&target_root).unwrap();
    let dir = tempfile::Builder::new()
        .prefix("show-order-")
        .tempdir_in(target_root)
        .unwrap();

    common::git_init_with_commit(dir.path());
    common::setup_dev_doc(dir.path(), "DEV", "fast");

    let doc_root = dir
        .path()
        .join(".dev-doc")
        .join(common::default_branch(dir.path()));
    fs::write(
        doc_root.join("task/task_2026-10-17_1.md"),
        "---\ntitle: Task - show ordering\nnums: 1\n---\n\n- [ ] TASK-T001: Task item\n  - type: feat\n  - priority: P1\n  - refs: test\n  - files:\n      create: []\n      modify: [src/a.rs]\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - visible\n",
    )
    .unwrap();
    fs::write(
        doc_root.join("issue/issue_test_2026-10-17_1.md"),
        "---\nsource: test\nnums: 1\n---\n\n- [ ] ISSUE-I001: Issue item\n  - severity: P1\n  - location: src/main.rs:1\n  - description: Issue description\n  - reproduce: Reproduce steps\n  - files_modify: [src/a.rs]\n  - fix:\n",
    )
    .unwrap();

    dir
}

fn run_dow(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn assert_id_title_are_first_two(output: &str, id: &str) {
    let id_token = format!("\"id\": \"{}\"", id);
    let lines: Vec<&str> = output.lines().collect();
    let id_line = lines
        .iter()
        .position(|line| line.contains(&id_token))
        .unwrap();
    let id_indent = lines[id_line].len() - lines[id_line].trim_start().len();
    let object_line = (0..id_line)
        .rev()
        .find(|index| {
            let line = lines[*index];
            line.trim() == "{" && line.len() - line.trim_start().len() + 2 == id_indent
        })
        .unwrap();
    let keys_before_id: Vec<&str> = lines[object_line + 1..id_line]
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    assert!(
        keys_before_id.is_empty(),
        "id must be the first object field; preceding fields: {:?}",
        keys_before_id
    );

    let next_line = lines[id_line + 1].trim_start();
    assert!(
        next_line.starts_with("\"title\":"),
        "title must be the second object field, got: {}",
        next_line
    );
}

#[test]
fn show_serializes_id_and_title_first_by_default_and_with_fields() {
    let dir = isolated_test_project();

    for args in [
        vec!["show", "T001", "I001"],
        vec!["show", "T001", "I001", "--fields", "files,done_when"],
    ] {
        let output = run_dow(dir.path(), &args);
        assert!(
            output.status.success(),
            "dow {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_id_title_are_first_two(&stdout, "TASK-T001");
        assert_id_title_are_first_two(&stdout, "ISSUE-I001");
    }
}
