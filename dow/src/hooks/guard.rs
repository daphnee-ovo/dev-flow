// dow/src/hooks/
// ├── guard.rs  -- dow hooks guard（文件写入守护）
//    合并 block-system-tmp.sh + block-non-dev-edit.sh

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use std::path::Path;

pub fn run(file: String) -> Result<i32, DowError> {
    // block-system-tmp: 阻止写入系统临时目录
    if is_system_tmp(&file) {
        println!("[dev-flow] BLOCKED: 禁止写入系统临时目录：{}", file);
        println!("→ 请使用项目内的 tmp/ 或 temp/ 目录。");
        return Ok(1);
    }

    // block-non-dev-edit: 非 DEV 阶段阻止代码编辑
    if is_code_file(&file) {
        if let Some(reason) = check_non_dev_block(&file) {
            println!("{}", reason);
            return Ok(1);
        }
    }

    Ok(0)
}

fn is_system_tmp(file: &str) -> bool {
    let path = Path::new(file);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(path)
    };

    let abs_str = abs.to_string_lossy();
    abs_str.starts_with("/tmp/")
        || abs_str.starts_with("/var/tmp/")
        || abs_str.starts_with("/dev/shm/")
        || abs_str.contains("/System/")
}

fn is_code_file(file: &str) -> bool {
    let code_exts = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".rs", ".go", ".java",
        ".rb", ".php", ".vue", ".svelte", ".sh",
    ];
    code_exts.iter().any(|ext| file.ends_with(ext))
}

fn check_non_dev_block(file: &str) -> Option<String> {
    // dev-doc 内的文件始终允许
    if file.starts_with("dev-doc/") || file.starts_with("dev-doc\\") {
        return None;
    }
    // tests/ 始终允许
    if file.starts_with("tests/") || file.starts_with("tests\\") {
        return None;
    }

    if !Path::new("dev-doc").is_dir() {
        return None;
    }

    let doc_root_path = doc_root::resolve("dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return None;
    }

    let phase = yaml::get(&status_file, "phase").ok().flatten().unwrap_or_default();

    if phase != "DEV" && phase != "TEST" {
        Some(format!(
            "[dev-flow] BLOCKED: 当前阶段为 {}，禁止编辑代码文件：{}\n→ 请先完成 {} 阶段文档，进入 DEV 后再编辑代码。",
            phase, file, phase
        ))
    } else {
        None
    }
}
