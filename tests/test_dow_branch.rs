// tests/
// ├── test_dow_branch.rs  -- 分支隔离集成测试

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

// 在项目内 tmp/ 下创建测试目录（避免 /tmp 被 guard 拦截）
static TEST_SEQ: AtomicU32 = AtomicU32::new(0);

fn create_test_dir() -> PathBuf {
    let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("tmp/test_branch");
    let dir = base.join(format!("t{}", seq));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn default_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn setup_git_repo(dir: &Path) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    fs::write(dir.join("dummy.txt"), "init").unwrap();
    Command::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn setup_branch_env(dir: &Path) {
    setup_git_repo(dir);
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
    // DEV 阶段需要至少一个未完成 task 避免 BLOCKED
    fs::write(
        doc.join("task/task_2026-05-26_1.md"),
        "- [ ] TASK-T001: active work\n  - priority: P1\n",
    ).unwrap();
}

#[test]
fn test_context_includes_branch_field() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let branch = json["branch"].as_str().unwrap();
    assert!(!branch.is_empty());
    assert_eq!(branch, default_branch(&dir));
}

#[test]
fn test_context_doc_root_matches_branch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    let branch = default_branch(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let doc_root = json["doc_root"].as_str().unwrap();
    assert_eq!(doc_root, format!(".dev-doc/{}", branch));
}

#[test]
fn test_guard_blocks_cross_branch_write() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    // 创建另一个分支目录模拟已有分支（需含 STATUS.yaml 才能被识别为分支）
    fs::create_dir_all(&dir.join(".dev-doc/feature-x")).unwrap();
    fs::write(dir.join(".dev-doc/feature-x/STATUS.yaml"), "name: test\nphase: DEV\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", ".dev-doc/feature-x/SPEC.md"])
        .current_dir(&dir)
        .output()
        .unwrap();

    // deny() exits 0 per Claude Code hook protocol, check stdout for BLOCKED
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BLOCKED"), "expected BLOCKED in: {}", stdout);
    assert!(stdout.contains("feature-x"));
}

#[test]
fn test_guard_allows_current_branch_write() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    let branch = default_branch(&dir);

    let file = format!(".dev-doc/{}/task/task_01.md", branch);
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", &file])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_auto_creates_branch_dir_on_new_branch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    // 创建新分支
    Command::new("git")
        .args(["checkout", "-b", "feat-new"])
        .current_dir(&dir)
        .output()
        .unwrap();

    // context 应该自动创建新分支目录
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["branch"], "feat-new");
    assert_eq!(json["doc_root"], ".dev-doc/feat-new");

    // 目录应该被自动创建
    assert!(&dir.join(".dev-doc/feat-new/STATUS.yaml").exists());
}

#[test]
fn test_new_branch_inherits_project_name() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    Command::new("git")
        .args(["checkout", "-b", "fix-bug"])
        .current_dir(&dir)
        .output()
        .unwrap();

    // 触发自动创建
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    let content = fs::read_to_string(&dir.join(".dev-doc/fix-bug/STATUS.yaml")).unwrap();
    assert!(content.contains("name: test"));
}

#[test]
fn test_guard_allows_non_devdoc_files() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    // 非 .dev-doc 非代码文件不受分支隔离限制
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "README.md"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_guard_blocks_bash_redirect_cross_branch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    fs::create_dir_all(dir.join(".dev-doc/other-branch")).unwrap();
    fs::write(dir.join(".dev-doc/other-branch/STATUS.yaml"), "name: test\nphase: DEV\n").unwrap();

    // 模拟 Bash 工具通过 TOOL_INPUT 环境变量传入命令
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", r#"{"command":"echo x > .dev-doc/other-branch/SPEC.md"}"#)
        .current_dir(&dir)
        .output()
        .unwrap();

    // deny() exits 0 per Claude Code hook protocol, check stdout for BLOCKED
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BLOCKED"), "expected BLOCKED in: {}", stdout);
}

#[test]
fn test_guard_allows_bash_redirect_current_branch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    let branch = default_branch(&dir);

    let cmd = format!(r#"{{"command":"echo x > .dev-doc/{}/task/new.md"}}"#, branch);
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", &cmd)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_post_bash_detects_branch_switch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "post-bash", "git checkout -b test-detect"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("检测到分支切换"));
}

#[test]
fn test_post_bash_ignores_non_git_commands() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "post-bash", "ls -la"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty());
}

// --- DEV 阶段无活跃工作回退测试 ---

/// 辅助：创建 DEV 阶段环境，所有 task 已完成
fn setup_dev_all_done(dir: &Path) {
    setup_git_repo(dir);
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: fast\nupdated: 2026-05-30 10:00\nstarted: 2026-05-30 09:00\n",
    ).unwrap();
    fs::write(
        doc.join("task/task_2026-05-30_1.md"),
        "- [x] TASK-T001: completed task\n  - priority: P1\n",
    ).unwrap();
}

#[test]
fn test_context_blocks_when_all_tasks_done() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["decision"], "block");
    let reason = json["reason"].as_str().unwrap();
    assert!(reason.contains("/task"), "should suggest /task");
    assert!(reason.contains("/issue"), "should suggest /issue");
    assert!(reason.contains("/test"), "should suggest /test");
}

#[test]
fn test_context_blocks_when_no_task_files() {
    let dir = create_test_dir();
    setup_git_repo(&dir);
    let branch = default_branch(&dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: fast\nupdated: 2026-05-30 10:00\nstarted: 2026-05-30 09:00\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["decision"], "block");
}

#[test]
fn test_context_does_not_block_with_undone_task() {
    let dir = create_test_dir();
    setup_branch_env(&dir); // has undone task

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("decision").is_none(), "should NOT block with active task");
    assert!(json.get("phase").is_some(), "should output full context");
}

#[test]
fn test_context_does_not_block_with_open_issue() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);
    let branch = default_branch(&dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::write(
        doc.join("issue/issue_test_2026-05-30_1.md"),
        "- [ ] BUG: something broken\n  - severity: P1\n",
    ).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("decision").is_none(), "should NOT block with open issue");
}

#[test]
fn test_guard_blocks_code_write_when_all_done() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = r#"{"tool_name":"Write","tool_input":{"file_path":"src/main.rs"}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("permissionDecision"), "should output permission JSON");
    assert!(stdout.contains("deny"), "should deny code write");
    assert!(stdout.contains("/task"), "should suggest /task");
}

#[test]
fn test_guard_allows_devdoc_edit_when_all_done() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);
    let branch = default_branch(&dir);

    // 编辑已存在的 task 文件应当允许（已存在文件不受 direct-create 拦截）
    let file_path = format!(".dev-doc/{}/task/task_2026-05-30_1.md", branch);
    let tool_input = format!(
        r#"{{"tool_name":"Edit","tool_input":{{"file_path":"{}"}}}}"#,
        file_path
    );
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", &tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("deny"), "should allow .dev-doc edit: {}", stdout);
}

#[test]
fn test_guard_allows_code_write_with_active_task() {
    let dir = create_test_dir();
    setup_branch_env(&dir); // has undone task

    let tool_input = r#"{"tool_name":"Write","tool_input":{"file_path":"src/main.rs"}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("deny"), "should allow code write with active task: {}", stdout);
}
