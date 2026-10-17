// tests/
// ├── test_guard_bash_targets.rs  -- Bash 写入目标解析回归测试

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::json;

// 在项目内 tmp/ 下创建测试目录（避免 /tmp 被 guard 拦截）
static TEST_SEQ: AtomicU32 = AtomicU32::new(0);

fn create_test_dir() -> PathBuf {
    let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp/test_guard_bash_targets");
    let dir = base.join(format!("t{}", seq));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn current_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// 最小 dev-flow 项目：git 仓库 + .dev-doc/<branch>/STATUS.yaml + docs/ tmp/ src/
fn setup_project(dir: &Path, phase: &str) -> String {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "test"]);
    fs::write(dir.join("README.md"), "init").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-qm", "init"]);

    let branch = current_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        format!(
            "name: test\nphase: {}\nmode: fast\nupdated: 2026-10-17 10:00\nstarted: 2026-10-17 09:00\n",
            phase
        ),
    )
    .unwrap();
    branch
}

/// 以 Bash hook payload 运行 guard，返回 stdout（Codex hook 协议 JSON）
fn run_guard(dir: &Path, command: &str) -> String {
    let payload = json!({
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", payload)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "guard hook must exit 0");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn heredoc_write_into_allowed_directory_is_allowed() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    let stdout = run_guard(&dir, "cat <<'EOF' > tmp/notes.md\n# notes\nEOF\n");
    assert!(
        !stdout.contains("BLOCKED"),
        "heredoc write into tmp/ must be allowed, got: {}",
        stdout
    );
}

#[test]
fn heredoc_body_is_not_scanned_as_shell_syntax() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    // 正文里的命令替换、括号、不配对引号与伪重定向都不该被当成 shell 语法
    let command = "cat <<'EOF' > docs/design.md\n$(cmd) ( unbalanced \" quote > src/evil.rs\nEOF\n";
    let stdout = run_guard(&dir, command);
    assert!(
        !stdout.contains("BLOCKED"),
        "heredoc body must not be scanned as shell syntax, got: {}",
        stdout
    );
}

#[test]
fn heredoc_write_outside_allowed_paths_reports_the_real_target() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    let stdout = run_guard(&dir, "cat <<'EOF' > src/main.rs\nfn main() {}\nEOF\n");
    assert!(stdout.contains("BLOCKED"), "expected deny, got: {}", stdout);
    assert!(
        stdout.contains("src/main.rs"),
        "deny reason must name the real target, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("cat <<"),
        "deny reason must not interpolate the raw command, got: {}",
        stdout
    );
}

#[test]
fn tab_stripping_heredoc_is_supported() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    let stdout = run_guard(&dir, "cat <<-EOF > tmp/notes.md\n\tbody\n\tEOF\n");
    assert!(
        !stdout.contains("BLOCKED"),
        "tab-stripping heredoc must be allowed, got: {}",
        stdout
    );
}

#[test]
fn herestring_is_not_treated_as_a_write_target() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    let stdout = run_guard(&dir, "wc -c <<< 'hello'\n");
    assert!(
        !stdout.contains("BLOCKED"),
        "herestring must not become a write target, got: {}",
        stdout
    );
}

#[test]
fn unterminated_heredoc_is_denied_without_a_fabricated_path() {
    let dir = create_test_dir();
    setup_project(&dir, "TASK");

    let stdout = run_guard(&dir, "cat <<'EOF' > tmp/notes.md\nnever terminated\n");
    assert!(stdout.contains("BLOCKED"), "expected deny, got: {}", stdout);
    assert!(
        !stdout.contains("cat <<"),
        "deny reason must not interpolate the raw command, got: {}",
        stdout
    );
}

#[test]
fn heredoc_write_respects_declared_scope_in_dev() {
    let dir = create_test_dir();
    let branch = setup_project(&dir, "DEV");
    fs::write(
        dir.join(".dev-doc")
            .join(&branch)
            .join("task/task_2026-10-17_1.md"),
        "- [ ] TASK-T001: heredoc scope\n  - priority: P1\n  - files:\n      create: []\n      modify: [\"src/main.rs\"]\n      test: []\n",
    )
    .unwrap();

    let claim = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["claim", "TASK-T001"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(claim.status.success(), "claim must succeed");

    let allowed = run_guard(&dir, "cat <<'EOF' > src/main.rs\nbody\nEOF\n");
    assert!(
        !allowed.contains("BLOCKED"),
        "declared scope must be honoured for heredocs, got: {}",
        allowed
    );

    let denied = run_guard(&dir, "cat <<'EOF' > src/other.rs\nbody\nEOF\n");
    assert!(
        denied.contains("BLOCKED"),
        "out of scope heredoc write must be denied, got: {}",
        denied
    );
    assert!(
        denied.contains("src/other.rs"),
        "deny reason must name the real path, got: {}",
        denied
    );
    assert!(
        !denied.contains("cat <<"),
        "deny reason must not interpolate the raw command, got: {}",
        denied
    );

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["claim", "--revoke"])
        .current_dir(&dir)
        .output()
        .unwrap();
}
