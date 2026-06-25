// tests/
// ├── test_guard_phase.rs  -- guard 阶段性写入控制集成测试

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use common::default_branch;

// 在项目内 tmp/ 下创建测试目录（避免 /tmp 被 guard 拦截）
static TEST_SEQ: AtomicU32 = AtomicU32::new(0);

fn create_test_dir() -> PathBuf {
    let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp/test_guard_phase");
    let dir = base.join(format!("t{}", seq));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_env(dir: &Path, phase: &str, mode: &str) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    fs::write(dir.join("dummy.txt"), "init").unwrap();
    Command::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();

    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        format!(
            "name: test\nphase: {}\nmode: {}\nupdated: 2026-05-31 10:00\nstarted: 2026-05-31 09:00\n",
            phase, mode
        ),
    )
    .unwrap();
    // DEV 阶段需要活跃 task
    if phase == "DEV" {
        fs::write(
            doc.join("task/task_2026-05-31_1.md"),
            "- [ ] TASK-T001: active\n  - priority: P1\n",
        )
        .unwrap();
    }
}

fn run_guard(dir: &Path, target: &str) -> (String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", target])
        .current_dir(dir)
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn assert_allow(stdout: &str) {
    assert!(
        stdout.trim().is_empty(),
        "expected ALLOW (empty output), got: {}",
        stdout
    );
}

fn assert_deny(stdout: &str) {
    assert!(stdout.contains("\"permissionDecision\":\"deny\""), "expected deny, got: {}", stdout);
}

fn assert_ask(stdout: &str) {
    assert!(stdout.contains("\"permissionDecision\":\"ask\""), "expected ask, got: {}", stdout);
}

// ─── 非 DEV 阶段：tmp/ 写入 ────────────────────────────────────────────────

#[test]
fn test_prd_tmp_noncode_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "tmp/notes.md");
    assert_allow(&stdout);
}

#[test]
fn test_prd_tmp_code_asks() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "tmp/demo.py");
    assert_ask(&stdout);
}

#[test]
fn test_spec_tmp_code_asks() {
    let dir = create_test_dir();
    setup_env(&dir, "SPEC", "quick");
    let (stdout, _) = run_guard(&dir, "tmp/script.sh");
    assert_ask(&stdout);
}

#[test]
fn test_task_tmp_code_asks() {
    let dir = create_test_dir();
    setup_env(&dir, "TASK", "fast");
    let (stdout, _) = run_guard(&dir, "tmp/test.rs");
    assert_ask(&stdout);
}

#[test]
fn test_prd_tmp_html_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "tmp/demo.html");
    assert_allow(&stdout);
}

#[test]
fn test_prd_tmp_json_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "tmp/data.json");
    assert_allow(&stdout);
}

// ─── 非 DEV 阶段：docs/ 写入 ───────────────────────────────────────────────

#[test]
fn test_prd_docs_noncode_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "docs/design.md");
    assert_allow(&stdout);
}

#[test]
fn test_prd_docs_code_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "docs/script.py");
    assert_deny(&stdout);
}

#[test]
fn test_spec_docs_code_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "SPEC", "quick");
    let (stdout, _) = run_guard(&dir, "docs/tool.sh");
    assert_deny(&stdout);
}

// ─── 非 DEV 阶段：源码写入 ─────────────────────────────────────────────────

#[test]
fn test_prd_source_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "src/main.rs");
    assert_deny(&stdout);
}

#[test]
fn test_spec_source_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "SPEC", "quick");
    let (stdout, _) = run_guard(&dir, "lib/utils.ts");
    assert_deny(&stdout);
}

// ─── 非 DEV 阶段：AI 配置始终放行 ──────────────────────────────────────────

#[test]
fn test_prd_ai_config_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, ".claude/settings.json");
    assert_allow(&stdout);
}

#[test]
fn test_spec_codex_config_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "SPEC", "quick");
    let (stdout, _) = run_guard(&dir, ".codex/config.yaml");
    assert_allow(&stdout);
}

// ─── 路径穿越 ───────────────────────────────────────────────────────────────

#[test]
fn test_prd_traversal_tmp_to_src_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    let (stdout, _) = run_guard(&dir, "tmp/../src/main.rs");
    assert_deny(&stdout);
}

#[test]
fn test_prd_traversal_stays_in_tmp_asks() {
    let dir = create_test_dir();
    setup_env(&dir, "PRD", "full");
    // tmp/sub/../demo.py → tmp/demo.py（仍在 tmp 下）
    let (stdout, _) = run_guard(&dir, "tmp/sub/../demo.py");
    assert_ask(&stdout);
}

// ─── DEV 阶段：正常写入 ────────────────────────────────────────────────────

#[test]
fn test_dev_source_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "DEV", "quick");
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["claim", "T001"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let (stdout, _) = run_guard(&dir, "src/main.rs");
    assert_allow(&stdout);
}

#[test]
fn test_dev_tmp_code_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "DEV", "quick");
    let (stdout, _) = run_guard(&dir, "tmp/test.py");
    assert_allow(&stdout);
}

#[test]
fn test_dev_no_claim_denied_with_hint() {
    let dir = create_test_dir();
    setup_env(&dir, "DEV", "quick");
    // 有未完成 task 但没 claim → 应提示 claim
    let (stdout, _) = run_guard(&dir, "src/main.rs");
    assert_deny(&stdout);
    assert!(stdout.contains("none claimed"), "should hint about claim, got: {}", stdout);
}

// ─── DEV 阶段：无活跃工作 ──────────────────────────────────────────────────

#[test]
fn test_dev_no_active_work_denied() {
    let dir = create_test_dir();
    setup_env(&dir, "DEV", "quick");
    // 把 task 标记为已完成
    let branch = default_branch(&dir);
    let task_file = dir.join(".dev-doc").join(&branch).join("task/task_2026-05-31_1.md");
    fs::write(&task_file, "- [x] TASK-T001: done\n  - priority: P1\n").unwrap();

    let (stdout, _) = run_guard(&dir, "src/main.rs");
    assert_deny(&stdout);
    assert!(stdout.contains("no pending tasks or open issues"));
}

#[test]
fn test_dev_no_active_work_tmp_still_allowed() {
    let dir = create_test_dir();
    setup_env(&dir, "DEV", "quick");
    let branch = default_branch(&dir);
    let task_file = dir.join(".dev-doc").join(&branch).join("task/task_2026-05-31_1.md");
    fs::write(&task_file, "- [x] TASK-T001: done\n  - priority: P1\n").unwrap();

    // tmp/ 在 DEV 白名单中，即使无活跃工作也放行
    let (stdout, _) = run_guard(&dir, "tmp/scratch.py");
    assert_allow(&stdout);
}
