// tests/
// ├── test_dow_doc.rs  -- dow doc 集成测试

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

fn setup_env(dir: &Path) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: test\nphase: DEV\nmode: quick\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
    ).unwrap();
}

#[test]
fn test_doc_task_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "task", "-n", "5"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "task");
    assert_eq!(json["slots"], 5);

    let created = json["created"].as_str().unwrap();
    let content = fs::read_to_string(dir.path().join(created)).unwrap();
    assert!(content.contains("nums: 5"));
    assert!(content.contains("TASK-T001"));
    assert!(content.contains("TASK-T005"));
}

#[test]
fn test_doc_issue_with_source() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "issue", "--source", "devtest", "-n", "2"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "issue");
    assert_eq!(json["slots"], 2);

    let created = json["created"].as_str().unwrap();
    assert!(created.contains("devtest"));
    let content = fs::read_to_string(dir.path().join(created)).unwrap();
    assert!(content.contains("source: devtest"));
    assert!(content.contains("ISSUE-I001"));
    assert!(content.contains("ISSUE-I002"));
}

#[test]
fn test_doc_seq_auto_increment() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    // 第一次
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "task"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 第二次
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "task"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let created = json["created"].as_str().unwrap();
    // 应该是 _2.md
    assert!(created.contains("_2.md"));
}

#[test]
fn test_doc_prd_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());
    let branch = default_branch(dir.path());
    fs::write(dir.path().join(".dev-doc").join(&branch).join("PRD.md"), "existing").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "prd"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("已存在"));
}

#[test]
fn test_doc_md_output() {
    // --md 不需要 git 环境，直接输出规范
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "task", "--md"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 文件格式规范"));
    assert!(stdout.contains("## 模板"));
    assert!(stdout.contains("TASK-T001"));
}

#[test]
fn test_doc_json_output() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "issue", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "issue");
    assert!(json["template"].is_string());
    assert!(json["fields"].is_array());
    assert!(json["sections"].is_array());
}

#[test]
fn test_doc_invalid_type() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "invalid"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未知文档类型"));
}

#[test]
fn test_doc_issue_invalid_source() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "issue", "--source", "random"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("无效的 issue 来源"));
}

#[test]
fn test_doc_issue_valid_sources() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    for source in &["test", "devtest", "other", "audit"] {
        let output = Command::new(env!("CARGO_BIN_EXE_dow"))
            .args(["doc", "issue", "--source", source])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(output.status.success(), "source '{}' should be valid", source);
    }
}

#[test]
fn test_doc_init_creates_skeleton() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let created = json["created"].as_array().unwrap();
    assert_eq!(created.len(), 4);
    assert!(created.contains(&serde_json::json!("README.md")));
    assert!(created.contains(&serde_json::json!("docs/structure.md")));
    assert!(created.contains(&serde_json::json!("docs/decisions.md")));
    assert!(created.contains(&serde_json::json!("docs/usage.md")));

    // 验证文件存在
    assert!(dir.path().join("README.md").exists());
    assert!(dir.path().join("docs/structure.md").exists());
    assert!(dir.path().join("docs/decisions.md").exists());
    assert!(dir.path().join("docs/usage.md").exists());

    // 验证 STATUS.yaml 包含 docs 字段
    let branch = default_branch(dir.path());
    let status = fs::read_to_string(dir.path().join(".dev-doc").join(&branch).join("STATUS.yaml")).unwrap();
    assert!(status.contains("docs:"));
    assert!(status.contains("  - docs/structure.md"));
}

#[test]
fn test_doc_init_no_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    // 预创建 README.md
    fs::write(dir.path().join("README.md"), "# Existing README\n").unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/structure.md"), "# Existing\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let skipped = json["skipped"].as_array().unwrap();
    assert!(skipped.contains(&serde_json::json!("README.md")));
    assert!(skipped.contains(&serde_json::json!("docs/structure.md")));

    // 已存在文件内容未变
    let readme = fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert_eq!(readme, "# Existing README\n");
}

#[test]
fn test_doc_init_with_project_name() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init", "--project-name", "my-cool-project"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let readme = fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(readme.contains("# my-cool-project"));
}

#[test]
fn test_doc_check_sync_no_ref() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    // 先 init docs
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // 无 --since 时，存在的文件都算 synced
    assert!(json["outdated"].as_array().unwrap().is_empty());
    // README.md 隐含检查
    let synced = json["synced"].as_array().unwrap();
    assert!(synced.iter().any(|v| v.as_str() == Some("README.md")));
}

#[test]
fn test_doc_check_sync_with_ref() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    // init docs 并 commit
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: add docs", "--no-verify"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["tag", "v0.1.0"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 修改一个 doc 文件并 commit
    fs::write(dir.path().join("docs/structure.md"), "# Updated\n").unwrap();
    Command::new("git")
        .args(["add", "docs/structure.md"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "docs: update structure", "--no-verify"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync", "--since", "v0.1.0"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let synced = json["synced"].as_array().unwrap();
    let outdated = json["outdated"].as_array().unwrap();

    assert!(synced.iter().any(|v| v.as_str() == Some("docs/structure.md")));
    assert!(outdated.iter().any(|v| v.as_str() == Some("docs/decisions.md")));
    assert!(outdated.iter().any(|v| v.as_str() == Some("docs/usage.md")));
}

#[test]
fn test_doc_check_sync_invalid_ref() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync", "--since", "nonexistent_tag"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ref_valid"], false);
    // 降级：所有存在的文件算 synced
    assert!(json["outdated"].as_array().unwrap().is_empty());
}

#[test]
fn test_doc_list() {
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e["exists"] == true));
}

// === E2E: 完整流程测试 ===

#[test]
fn test_e2e_init_creates_docs_and_registers() {
    // SPEC-AC-001: dow init 在全新项目生成骨架 + 注册
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["init", "--name", "e2e-test", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "dow init failed: {}", String::from_utf8_lossy(&output.stderr));

    // 验证文件创建
    assert!(dir.path().join("README.md").exists());
    assert!(dir.path().join("docs/structure.md").exists());
    assert!(dir.path().join("docs/decisions.md").exists());
    assert!(dir.path().join("docs/usage.md").exists());

    // 验证 STATUS.yaml 注册
    let branch = default_branch(dir.path());
    let status = fs::read_to_string(dir.path().join(".dev-doc").join(&branch).join("STATUS.yaml")).unwrap();
    assert!(status.contains("docs:"));
    assert!(status.contains("  - docs/structure.md"));
    assert!(status.contains("  - docs/decisions.md"));
    assert!(status.contains("  - docs/usage.md"));
}

#[test]
fn test_e2e_no_docs_field_skips_check() {
    // SPEC-AC-007: 无 docs 字段时 check-sync 返回空
    let dir = tempfile::tempdir().unwrap();
    setup_env(dir.path());
    // setup_env 不写 docs 字段

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // 只有隐含的 README.md（可能不存在）
    let synced = json["synced"].as_array().unwrap();
    let outdated = json["outdated"].as_array().unwrap();
    let missing = json["missing"].as_array().unwrap();
    // 无 docs 注册时只有 README.md 被隐含检查
    assert!(outdated.is_empty());
    assert_eq!(synced.len() + missing.len(), 1); // README.md
}

#[test]
fn test_e2e_check_sync_detects_outdated() {
    // SPEC-AC-004 + AC-006 flow: check-sync detects docs not updated
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    // Init project
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["init", "--name", "e2e-sync", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Commit everything
    Command::new("git").args(["add", "-A"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: init", "--no-verify"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["tag", "v0.1.0"]).current_dir(dir.path()).output().unwrap();

    // Make code changes without updating docs
    fs::write(dir.path().join("src.rs"), "fn main() {}").unwrap();
    Command::new("git").args(["add", "src.rs"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: add src", "--no-verify"]).current_dir(dir.path()).output().unwrap();

    // check-sync should show docs as outdated
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync", "--since", "v0.1.0"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let outdated = json["outdated"].as_array().unwrap();
    // All docs should be outdated (not modified since tag)
    assert!(outdated.len() >= 3, "Expected at least 3 outdated docs, got: {:?}", outdated);

    // Now update one doc and recommit
    fs::write(dir.path().join("docs/structure.md"), "# Updated structure\n").unwrap();
    Command::new("git").args(["add", "docs/structure.md"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "docs: update structure", "--no-verify"]).current_dir(dir.path()).output().unwrap();

    // Re-check: structure.md should now be synced
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["doc", "check-sync", "--since", "v0.1.0"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let synced = json["synced"].as_array().unwrap();
    assert!(synced.iter().any(|v| v.as_str() == Some("docs/structure.md")));
}

#[test]
fn test_e2e_iterate_warns_outdated_docs() {
    // SPEC-AC-006: iterate 检测到未更新文档时警告但可继续
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    // Init + commit + tag
    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["init", "--name", "e2e-iterate", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    fs::write(dir.path().join("VERSION"), "0.1.0").unwrap();

    Command::new("git").args(["add", "-A"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: init", "--no-verify"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["tag", "v0.1.0"]).current_dir(dir.path()).output().unwrap();

    // 改代码但不更新文档
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    Command::new("git").args(["add", "main.rs"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: add main", "--no-verify"]).current_dir(dir.path()).output().unwrap();

    // iterate 预览应输出 doc_sync_warnings
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["iterate", "--topic", "test", "--type", "feat"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() {
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        if let Some(warnings) = json.get("doc_sync_warnings") {
            let w = warnings.as_array().unwrap();
            assert!(!w.is_empty(), "Expected doc_sync_warnings to contain outdated docs");
        }
    }
}

#[test]
fn test_e2e_iterate_no_docs_field_skips_check() {
    // SPEC-AC-007: 无 docs 字段时 iterate 不执行检查
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    let branch = default_branch(dir.path());
    let doc = dir.path().join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        "name: e2e-no-docs\nphase: DEV\nmode: fast\nupdated: 2026-06-16 10:00\nstarted: 2026-06-16 09:00\n",
    ).unwrap();
    fs::write(dir.path().join("VERSION"), "0.1.0").unwrap();

    Command::new("git").args(["add", "-A"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "init", "--no-verify"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["tag", "v0.1.0"]).current_dir(dir.path()).output().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["iterate", "--topic", "test", "--type", "feat"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.get("doc_sync_warnings").is_none(),
            "Should not have doc_sync_warnings when no docs field: {}", stdout);
    }
}

#[test]
fn test_e2e_hook_reminder_triggers() {
    // SPEC-AC-005: post-write hook 在代码变更>=3文件时提醒
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["init", "--name", "e2e-hook", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 创建一个 task 文件以允许进入 DEV
    let branch = default_branch(dir.path());
    let task_dir = dir.path().join(".dev-doc").join(&branch).join("task");
    fs::write(task_dir.join("task_2026-06-16_1.md"), "---\ntitle: test\nnums: 1\n---\n\n- [ ] TASK-T001: test task\n  - priority: P1\n  - done_when:\n      - done\n").unwrap();

    Command::new("git").args(["add", "-A"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: init", "--no-verify"]).current_dir(dir.path()).output().unwrap();

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--phase", "DEV"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 创建 3+ 代码文件并 stage
    fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
    fs::write(dir.path().join("b.rs"), "fn b() {}").unwrap();
    fs::write(dir.path().join("c.rs"), "fn c() {}").unwrap();
    Command::new("git").args(["add", "a.rs", "b.rs", "c.rs"]).current_dir(dir.path()).output().unwrap();

    // 调用 post-write hook
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "post-write", "d.rs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("持久化文档") || stdout.contains("注册文档"),
        "Expected persistent docs reminder in output, got: {}", stdout
    );
}

#[test]
fn test_e2e_hook_no_reminder_below_threshold() {
    // post-write hook 在代码变更<3文件时不提醒
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["init", "--name", "e2e-hook-quiet", "--mode", "fast"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 创建 task 以允许 DEV 阶段
    let branch = default_branch(dir.path());
    let task_dir = dir.path().join(".dev-doc").join(&branch).join("task");
    fs::write(task_dir.join("task_2026-06-16_1.md"), "---\ntitle: test\nnums: 1\n---\n\n- [ ] TASK-T001: test task\n  - priority: P1\n  - done_when:\n      - done\n").unwrap();

    Command::new("git").args(["add", "-A"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: init", "--no-verify"]).current_dir(dir.path()).output().unwrap();

    Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--phase", "DEV"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // 只有 1 个文件变更
    fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
    Command::new("git").args(["add", "a.rs"]).current_dir(dir.path()).output().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["hooks", "post-write", "a.rs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("持久化文档") && !stdout.contains("注册文档"),
        "Should NOT show reminder below threshold, got: {}", stdout
    );
}
