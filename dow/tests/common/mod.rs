// tests/common/mod.rs — 集成测试公共辅助函数

use std::fs;
use std::path::Path;
use std::process::Command;

/// 获取 git init 后的默认分支名
pub fn default_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// 初始化 git 仓库并创建初始 commit
#[allow(dead_code)]
pub fn git_init_with_commit(dir: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    fs::write(dir.join("dummy.txt"), "init").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

/// 创建 .dev-doc 基础结构（STATUS.yaml + task/ + issue/）
pub fn setup_dev_doc(dir: &Path, phase: &str, mode: &str) {
    let branch = default_branch(dir);
    let doc = dir.join(".dev-doc").join(&branch);
    fs::create_dir_all(doc.join("task")).unwrap();
    fs::create_dir_all(doc.join("issue")).unwrap();
    fs::write(
        doc.join("STATUS.yaml"),
        format!(
            "name: test\nphase: {}\nmode: {}\nupdated: 2026-05-26 10:00\nstarted: 2026-05-26 09:00\n",
            phase, mode
        ),
    )
    .unwrap();
}

/// 读取 dow status 的单个字段
#[allow(dead_code)]
pub fn read_status_field(dir: &Path, field: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["status", "--field", field])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
