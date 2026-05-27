// dow/src/hooks/
// ├── post_bash.rs  -- Bash 执行后检测分支切换

use crate::core::doc_root;
use crate::error::DowError;
use std::path::Path;

pub fn run(command: Option<String>) -> Result<i32, DowError> {
    if !Path::new("dev-doc").is_dir() {
        return Ok(0);
    }

    let cmd = command
        .or_else(|| {
            std::env::var("TOOL_INPUT").ok().and_then(|input| {
                serde_json::from_str::<serde_json::Value>(&input)
                    .ok()
                    .and_then(|j| j.get("command").and_then(|v| v.as_str()).map(String::from))
            })
        })
        .unwrap_or_default();

    if cmd.is_empty() {
        return Ok(0);
    }

    // 检测是否执行了分支切换命令
    if !is_branch_switch_command(&cmd) {
        return Ok(0);
    }

    // 获取当前分支并报告
    let branch = doc_root::current_branch().unwrap_or_else(|| "unknown".to_string());
    let doc_root_path = doc_root::resolve("dev-doc");

    println!(
        "[dev-flow] 检测到分支切换 → 当前分支：`{}`，doc_root：{}",
        branch,
        doc_root_path.display()
    );

    // 检查新分支是否有 STATUS.yaml（resolve 会自动创建，但这里给提示）
    if doc_root_path.join("STATUS.yaml").exists() {
        let phase = crate::core::yaml::get(&doc_root_path.join("STATUS.yaml"), "phase")
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());
        println!("  阶段：{}，文档目录已就绪。", phase);
    } else {
        println!("  ⚠ 新分支尚未初始化 dev-doc，将自动创建。");
    }

    Ok(0)
}

fn is_branch_switch_command(cmd: &str) -> bool {
    let patterns = [
        "git checkout",
        "git switch",
        "git checkout -b",
        "git switch -c",
    ];
    patterns.iter().any(|p| cmd.contains(p))
}
