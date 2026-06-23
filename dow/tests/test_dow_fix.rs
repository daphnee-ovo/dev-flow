// tests/
// ├── test_dow_fix.rs  -- dow fix 集成测试

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

fn setup_env(dir: &Path) -> std::path::PathBuf {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-06-23 10:00\nstarted: 2026-06-20 09:00\n",
    ).unwrap();
    fs::write(doc.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    fs::write(dir.join(".gitignore"), "tmp/\n").unwrap();
    doc
}

fn run_fix(dir: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .arg("fix")
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("failed to parse fix output: {:?}", String::from_utf8_lossy(&output.stdout)))
}

#[test]
fn test_fix_issue_closed_rename() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // 创建一个全部勾选的 issue 文件
    fs::write(
        doc.join("issue/issue_other_2026-06-20_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [x] ISSUE-I001：测试\n  - severity: P1\n  - location：a.rs:1\n  - description：test\n  - fix：done\n",
    ).unwrap();

    let result = run_fix(dir.path());
    let fixed: Vec<String> = result["fixed"].as_array().unwrap()
        .iter().map(|v| v.as_str().unwrap().to_string()).collect();

    assert!(fixed.iter().any(|f| f.contains("closed_")),
        "should rename to closed_ prefix, got: {:?}", fixed);

    // 文件应已重命名
    assert!(doc.join("issue/closed_issue_other_2026-06-20_1.md").exists());
    assert!(!doc.join("issue/issue_other_2026-06-20_1.md").exists());
}

#[test]
fn test_fix_task_done_rename() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // 创建一个全部勾选的 task 文件
    fs::write(
        doc.join("task/task_2026-06-20_1.md"),
        "---\ntitle: TASK - test\nnums: 1\n---\n\n- [x] TASK-T001: test task\n  - priority: P1\n  - complexity: S\n  - done_when: done\n",
    ).unwrap();

    let result = run_fix(dir.path());
    let fixed: Vec<String> = result["fixed"].as_array().unwrap()
        .iter().map(|v| v.as_str().unwrap().to_string()).collect();

    assert!(fixed.iter().any(|f| f.contains("done_")),
        "should rename to done_ prefix, got: {:?}", fixed);

    assert!(doc.join("task/done_task_2026-06-20_1.md").exists());
    assert!(!doc.join("task/task_2026-06-20_1.md").exists());
}

#[test]
fn test_fix_issue_renumber_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // 两个文件都从 ISSUE-I001 开始（模拟旧版本行为）
    fs::write(
        doc.join("issue/closed_issue_other_2026-06-17_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [x] ISSUE-I001：旧问题1\n  - severity: P1\n  - location：a.rs:1\n  - description：test\n  - fix：done\n",
    ).unwrap();
    fs::write(
        doc.join("issue/closed_issue_other_2026-06-18_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [x] ISSUE-I001：旧问题2\n  - severity: P1\n  - location：b.rs:1\n  - description：test\n  - fix：done\n",
    ).unwrap();

    let result = run_fix(dir.path());
    let fixed: Vec<String> = result["fixed"].as_array().unwrap()
        .iter().map(|v| v.as_str().unwrap().to_string()).collect();

    assert!(fixed.iter().any(|f| f.contains("重编号")),
        "should renumber issues, got: {:?}", fixed);

    // 第一个文件保持 I001，第二个变为 I002
    let content1 = fs::read_to_string(doc.join("issue/closed_issue_other_2026-06-17_1.md")).unwrap();
    let content2 = fs::read_to_string(doc.join("issue/closed_issue_other_2026-06-18_1.md")).unwrap();
    assert!(content1.contains("ISSUE-I001"), "first file should keep I001");
    assert!(content2.contains("ISSUE-I002"), "second file should become I002, got: {}", content2);
}

#[test]
fn test_fix_no_change_when_clean() {
    let dir = tempfile::tempdir().unwrap();
    let doc = setup_env(dir.path());

    // 正常的 open issue 不应被重命名
    fs::write(
        doc.join("issue/issue_other_2026-06-20_1.md"),
        "---\nsource: other\nnums: 1\n---\n\n- [ ] ISSUE-I001：未修复\n  - severity: P1\n  - location：a.rs:1\n  - description：test\n  - fix：\n",
    ).unwrap();

    let result = run_fix(dir.path());
    let fixed = result["fixed"].as_array().unwrap();
    let unfixable = result["unfixable"].as_array().unwrap();

    assert!(fixed.is_empty(), "no fixes expected, got: {:?}", fixed);
    assert!(unfixable.is_empty(), "no unfixable expected, got: {:?}", unfixable);
    assert!(doc.join("issue/issue_other_2026-06-20_1.md").exists());
}
