// dow/src/lib/
// ├── doc_root.rs  -- doc_root 解析逻辑（对应 devflow_resolve_doc_root）

use std::path::{Path, PathBuf};
use std::process::Command;

/// 解析实际的 doc_root 路径
/// 优先级：dev-doc/<branch>/STATUS.yaml > dev-doc/STATUS.yaml > dev-doc 内首个子目录
pub fn resolve(base: &str) -> PathBuf {
    let base_path = Path::new(base);

    // 尝试获取当前 git 分支
    if let Some(branch) = current_branch() {
        let branch_path = base_path.join(&branch);
        if branch_path.join("STATUS.yaml").exists() {
            return branch_path;
        }
    }

    // 直接在 base 下找 STATUS.yaml
    if base_path.join("STATUS.yaml").exists() {
        return base_path.to_path_buf();
    }

    // 搜索子目录
    if base_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| e.path().join("STATUS.yaml").exists())
                .map(|e| e.path())
                .collect();
            found.sort();
            if let Some(first) = found.first() {
                return first.clone();
            }
        }
    }

    base_path.to_path_buf()
}

fn current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}
