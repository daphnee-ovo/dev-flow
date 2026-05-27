// dow/src/lib/
// ├── doc_root.rs  -- doc_root 解析逻辑（对应 devflow_resolve_doc_root）

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 解析实际的 doc_root 路径
/// 强制使用 dev-doc/<branch>/ 格式（包括 main/master）
/// 新分支自动创建目录和 STATUS.yaml
pub fn resolve(base: &str) -> PathBuf {
    let base_path = Path::new(base);

    if let Some(branch) = current_branch() {
        let branch_path = base_path.join(&branch);
        if branch_path.join("STATUS.yaml").exists() {
            return branch_path;
        }
        // 自动创建分支目录
        if base_path.is_dir() {
            if let Ok(()) = fs::create_dir_all(&branch_path) {
                let status_content = format!(
                    "name: {}\nphase: PRD\nmode: fast\nupdated: {}\nstarted: {}\n",
                    read_project_name(base_path).unwrap_or_else(|| "project".to_string()),
                    now_str(),
                    now_str(),
                );
                let _ = fs::write(branch_path.join("STATUS.yaml"), &status_content);
                return branch_path;
            }
        }
    }

    // 回退：搜索子目录（无分支信息时）
    if base_path.is_dir() {
        if let Ok(entries) = fs::read_dir(base_path) {
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

/// 获取当前 git 分支名
pub fn current_branch() -> Option<String> {
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

/// 从已有分支目录的 STATUS.yaml 中读取项目名
fn read_project_name(base_path: &Path) -> Option<String> {
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let status = entry.path().join("STATUS.yaml");
            if status.exists() {
                if let Ok(content) = fs::read_to_string(&status) {
                    for line in content.lines() {
                        if let Some(name) = line.strip_prefix("name:") {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn now_str() -> String {
    let output = Command::new("date")
        .args(["+%Y-%m-%d %H:%M"])
        .output()
        .ok();
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
