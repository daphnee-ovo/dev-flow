// tests/
// ├── test_dow_branch.rs  -- 分支隔离集成测试

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
        .join("tmp/test_branch");
    let dir = base.join(format!("t{}", seq));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_git_repo(dir: &Path) {
    common::git_init_with_commit(dir);
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
    )
    .unwrap();
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
    // doc_root removed from output; verify branch is correct instead
    assert_eq!(json["branch"], branch);
}

#[test]
fn test_context_codex_hook_wraps_context_json() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    let branch = default_branch(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context", "--codex-hook"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let hook_output = &json["hookSpecificOutput"];
    assert_eq!(hook_output["hookEventName"], "UserPromptSubmit");

    let context_json = hook_output["additionalContext"].as_str().unwrap();
    let context: serde_json::Value = serde_json::from_str(context_json).unwrap();
    assert_eq!(context["branch"], branch);
    assert_eq!(context["mode"], "quick");
    assert_eq!(context["phase"], "DEV");
}

#[test]
fn test_guard_blocks_cross_branch_write() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    // 创建另一个分支目录模拟已有分支（需含 STATUS.yaml 才能被识别为分支）
    fs::create_dir_all(&dir.join(".dev-doc/feature-x")).unwrap();
    fs::write(
        dir.join(".dev-doc/feature-x/STATUS.yaml"),
        "name: test\nphase: DEV\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", ".dev-doc/feature-x/SPEC.md"])
        .current_dir(&dir)
        .output()
        .unwrap();

    // deny() exits 0 per Claude Code hook protocol, check stdout for BLOCKED
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BLOCKED"),
        "expected BLOCKED in: {}",
        stdout
    );
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
    fs::write(
        dir.join(".dev-doc/other-branch/STATUS.yaml"),
        "name: test\nphase: DEV\n",
    )
    .unwrap();

    // 模拟 Bash 工具通过 TOOL_INPUT 环境变量传入命令
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env(
            "TOOL_INPUT",
            r#"{"command":"echo x > .dev-doc/other-branch/SPEC.md"}"#,
        )
        .current_dir(&dir)
        .output()
        .unwrap();

    // deny() exits 0 per Claude Code hook protocol, check stdout for BLOCKED
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BLOCKED"),
        "expected BLOCKED in: {}",
        stdout
    );
}

#[test]
fn test_guard_allows_bash_redirect_current_branch() {
    let dir = create_test_dir();
    setup_branch_env(&dir);
    let branch = default_branch(&dir);

    let cmd = format!(
        r#"{{"command":"echo x > .dev-doc/{}/task/new.md"}}"#,
        branch
    );
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
    assert!(stdout.contains("Branch switch detected"));
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
    )
    .unwrap();
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
    assert!(
        reason.contains("no pending tasks or open issues"),
        "should mention no pending work, got: {}",
        reason
    );
    assert!(
        reason.contains("create a task/issue"),
        "should suggest creating task/issue, got: {}",
        reason
    );
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
fn test_context_codex_hook_injects_context_without_blocking() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context", "--codex-hook"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("decision").is_none());
    assert!(json.get("reason").is_none());
    assert_eq!(
        json["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );

    let context_json = json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    let context: serde_json::Value = serde_json::from_str(context_json).unwrap();
    assert_eq!(context["blocked"], true);
    assert!(context["guard_notice"].as_str().is_some());
    assert!(
        context["reason"]
            .as_str()
            .unwrap()
            .contains("no pending tasks or open issues"),
        "should mention no pending work in codex context"
    );
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
    assert!(
        json.get("decision").is_none(),
        "should NOT block with active task"
    );
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
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "context"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json.get("decision").is_none(),
        "should NOT block with open issue"
    );
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
    assert!(
        stdout.contains("permissionDecision"),
        "should output permission JSON"
    );
    assert!(stdout.contains("deny"), "should deny code write");
    assert!(
        stdout.contains("no pending tasks or open issues"),
        "should mention no pending work, got: {}",
        stdout
    );
}

#[test]
fn test_guard_allows_task_create_metadata_paths_without_claim() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow task create --title 'fix example' --file '{\"modify\":[\"src/example.rs\"],\"create\":[\"src/generated.rs\"]}'"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "metadata paths must not be treated as writes: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_flow_create_with_prose_metadata() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow task create --title 'fix example' --refs 'GitHub issue prose: tee cp mv sed perl dd and > are data' --file '{\"modify\":[\"dow/src/hooks/guard.rs\"],\"test\":[\"dow/tests/test_dow_branch.rs\"]}' --complexity M --done-when 'criterion one,criterion two'"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "flow metadata must not be treated as writes: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_issue_create_prose_metadata_without_claim() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow issue create --title 'guard issue' --severity P1 --location 'dow/src/hooks/guard.rs' --desc 'The prose contains tee cp mv sed perl dd and > as ordinary text.' --reproduce 'Run the command and observe the guard.' --source other --file '{\"modify\":[\"dow/src/hooks/guard.rs\"]}'"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "issue metadata must not be treated as writes: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_github_issue_body_prose_without_filesystem_target() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "gh issue create --title 'guard issue' --body '## Summary\n> ordinary prose mentions tee cp mv sed perl dd and a path-like word to\n'"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "GitHub issue prose must not be treated as a write: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_blocks_known_filesystem_commands() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let commands = [
        "cp source.txt output.txt",
        "mv source.txt output.txt",
        "sed -i 's/old/new/' src/main.rs",
        "perl -i -pe 's/old/new/' src/main.rs",
        "dd if=source.bin of=output.bin",
    ];

    for command in commands {
        let tool_input = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": command}
        });
        let output = Command::new(env!("CARGO_BIN_EXE_dow"))
            .args(["hooks", "guard", "--codex-hook"])
            .env("TOOL_INPUT", tool_input.to_string())
            .current_dir(&dir)
            .output()
            .unwrap();

        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("\"permissionDecision\":\"deny\""),
            "known filesystem command was not blocked: {command}\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn test_guard_apply_patch_reads_only_file_markers() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let patch = "*** Begin Patch\n*** Update File: tmp/update.md\n@@\n- sidebar WebView -> MainSidebar\n+ sidebar WebView -> MainSidebar\n*** Add File: tmp/add.md\n+new content\n*** Delete File: tmp/delete.md\n*** End Patch";
    let tool_input = serde_json::json!({
        "tool_name": "apply_patch",
        "tool_input": {"patch": patch}
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "patch body text must not become a Bash redirect target: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_apply_patch_command_field_does_not_use_bash_parser() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let patch = "*** Begin Patch\n*** Update File: tmp/patch.md\n@@\n sidebar WebView -> MainSidebar\n*** End Patch";
    let tool_input = serde_json::json!({
        "tool_name": "apply_patch",
        "tool_input": {"command": patch}
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "apply_patch command text must not be parsed as Bash: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_task_create_refs_containing_tee_word() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow task create --title 'fix example' --priority P0 --refs 'User-defined send steer queue interaction-group semantics' --file '{\"modify\":[\"crates/rozsa-app/src/agent_session.rs\",\"crates/rozsa-gui/src/commands.rs\"],\"test\":[\"crates/rozsa-gui/tests/turn_diff_test.rs\"]}' --complexity L --done-when 'Interaction summary works'"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "refs containing `steer` must not create a fake tee target: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_stdin_json_task_create_metadata_paths_without_claim() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let command = "printf '%s' '{\"title\":\"fix example\",\"refs\":\"issue prose: tee cp mv sed perl dd and > are data\",\"files\":{\"modify\":[\"src/example.rs\"]}}' | dow task create";
    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": command}
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdin metadata paths must not be treated as writes: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_positional_trusted_flow_command() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args([
            "hooks",
            "guard",
            "dow task create --file '{\"modify\":[\"src/example.rs\"]}'",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_allows_absolute_current_dow_metadata_command() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);
    let command = format!(
        "{} task create --file '{{\"modify\":[\"src/example.rs\"]}}'",
        env!("CARGO_BIN_EXE_dow")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", &command])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_guard_does_not_exempt_flow_command_with_redirect() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow task create --file '{\"modify\":[\"src/example.rs\"]}' > output"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"permissionDecision\":\"deny\""),
        "got: {}",
        stdout
    );
}

#[test]
fn test_guard_does_not_exempt_flow_command_with_tee() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "dow task create --file '{\"modify\":[\"src/example.rs\"]}' | tee output"
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input.to_string())
        .current_dir(&dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"permissionDecision\":\"deny\""),
        "got: {}",
        stdout
    );
}

#[test]
fn test_guard_accepts_codex_hook_global_arg_after_subcommand() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);

    let tool_input = r#"{"tool_name":"Write","tool_input":{"file_path":"src/main.rs"}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard", "--codex-hook"])
        .env("TOOL_INPUT", tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("permissionDecision"),
        "should output permission JSON"
    );
    assert!(stdout.contains("deny"), "should deny code write");
}

#[test]
fn test_guard_blocks_devdoc_task_edit() {
    let dir = create_test_dir();
    setup_dev_all_done(&dir);
    let branch = default_branch(&dir);

    // 编辑已存在的 task 文件应当拦截（结构型文件全链路走 dow 命令）
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
    assert!(
        stdout.contains("deny"),
        "should deny task file edit: {}",
        stdout
    );
    assert!(
        stdout.contains("dow task done"),
        "should suggest dow task done: {}",
        stdout
    );
}

#[test]
fn test_guard_allows_code_write_with_active_task() {
    let dir = create_test_dir();
    setup_branch_env(&dir); // has undone task

    // Add file scope to the task so guard allows write
    let branch = default_branch(&dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::write(
        doc.join("task/task_2026-05-26_1.md"),
        "- [ ] TASK-T001: active work\n  - priority: P1\n  - files:\n      modify: src/main.rs\n",
    )
    .unwrap();

    // claim task so guard allows write
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["claim", "T001"])
        .current_dir(&dir)
        .output()
        .unwrap();

    let tool_input = r#"{"tool_name":"Write","tool_input":{"file_path":"src/main.rs"}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "guard"])
        .env("TOOL_INPUT", tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("deny"),
        "should allow code write with active task: {}",
        stdout
    );
}

#[test]
fn test_post_bash_accepts_codex_hook_global_arg_after_subcommand() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let tool_input = r#"{"tool_name":"Bash","tool_input":{"command":"git switch main"}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "post-bash", "--codex-hook"])
        .env("TOOL_INPUT", tool_input)
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_save_changelog_codex_hook_outputs_stop_json() {
    let dir = create_test_dir();
    setup_branch_env(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "session-stop", "--codex-hook"])
        .current_dir(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json, serde_json::json!({}));
    assert!(output.stderr.is_empty());

    let has_changelog_entry = fs::read_dir(dir.join(".dev-doc"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("CHANGELOG.md"))
        .filter_map(|path| fs::read_to_string(path).ok())
        .any(|content| content.contains("feat: active work"));
    assert!(has_changelog_entry);
}
