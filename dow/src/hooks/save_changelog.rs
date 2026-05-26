// dow/src/hooks/
// ├── save_changelog.rs  -- dow hooks save-changelog（会话结束时追加记录）

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run() -> Result<i32, DowError> {
    if !Path::new("dev-doc").is_dir() {
        return Ok(0);
    }

    let doc_root_path = doc_root::resolve("dev-doc");
    let changelog = doc_root_path.join("CHANGELOG.md");
    let date = Local::now().format("%Y-%m-%d").to_string();
    let time = Local::now().format("%H:%M").to_string();

    // 推断 topic：最近 git commit message 或 phase
    let topic = infer_topic(&doc_root_path);

    // 创建 CHANGELOG（如果不存在）
    if !changelog.exists() {
        fs::write(&changelog, "# Changelog\n\n")
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let mut content = fs::read_to_string(&changelog)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    // 检查是否已有当天日期段
    let date_header = format!("## {}", date);
    if !content.contains(&date_header) {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("\n{}\n", date_header));
    }

    // 去重检查
    let entry = format!("- {} {}", time, topic);
    if content.contains(&entry) {
        return Ok(0);
    }

    // 追加条目
    content.push_str(&format!("{}\n", entry));
    fs::write(&changelog, content)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    println!("[dev-flow] CHANGELOG 已更新：{} {}", time, topic);
    Ok(0)
}

fn infer_topic(doc_root: &Path) -> String {
    // 优先用最近的 git commit message
    if let Ok(output) = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .output()
    {
        if output.status.success() {
            let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // 去掉 hash 前缀
            if let Some(space_pos) = msg.find(' ') {
                let topic = msg[space_pos + 1..].to_string();
                if !topic.is_empty() {
                    return topic;
                }
            }
        }
    }

    // fallback: 用 phase
    let status_file = doc_root.join("STATUS.yaml");
    if let Ok(Some(phase)) = yaml::get(&status_file, "phase") {
        return phase.to_lowercase();
    }

    "session".to_string()
}
