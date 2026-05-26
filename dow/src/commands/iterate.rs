// dow/src/commands/
// ├── iterate.rs  -- dow iterate（迭代交付：校验 → 归档 → commit + tag → bump）

use crate::cli::IterateArgs;
use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct IterateOutput {
    released_version: String,
    tag: String,
    archive_dir: String,
    archived_files: Vec<String>,
    next_version: String,
    next_phase: String,
}

pub fn run(args: IterateArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve("dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new("STATUS.yaml 不存在", 1));
    }

    let mode = yaml::get(&status_file, "mode")
        .map_err(|e| DowError::new(e.to_string(), 1))?
        .unwrap_or_else(|| "quick".to_string());

    let effective_mode = if mode.starts_with("audit/") {
        mode[6..].to_string()
    } else {
        mode.clone()
    };

    // 1. 校验：任务完成度（audit 模式跳过）
    if !mode.starts_with("audit/") {
        let (total, done) = count_tasks(&doc_root_path);
        if total > 0 && done < total {
            return Err(DowError::new(
                format!("任务未全部完成（{}/{}）", done, total),
                1,
            ));
        }
    }

    // 2. 校验：无 open P0 issue
    let p0_open = count_p0_issues(&doc_root_path);
    if p0_open > 0 {
        return Err(DowError::new(
            format!("有 {} 个未关闭的 P0 issue", p0_open),
            1,
        ));
    }

    // 3. 读取版本
    let version = read_version()?;
    let new_version = bump_version(&version, &args.bump)?;

    // 4. 计算归档内容
    let archive_dir = format!(
        "{}/archive/v{}-{}",
        doc_root_path.to_string_lossy(),
        version,
        args.topic
    );
    let archived_files = list_archive_files(&doc_root_path);

    // --view 模式：只预览
    if args.view {
        let result = IterateOutput {
            tag: format!("v{}", &version),
            released_version: version.clone(),
            archive_dir,
            archived_files,
            next_version: new_version,
            next_phase: next_phase(&effective_mode, &mode),
        };
        if human {
            print_human_preview(&result);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    // 5. 执行归档
    let archive_path = Path::new(&archive_dir);
    if archive_path.exists() {
        return Err(DowError::new(
            format!("归档目录已存在：{}", archive_dir),
            1,
        ));
    }
    fs::create_dir_all(archive_path.join("issue"))
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    move_archive_files(&doc_root_path, archive_path)?;

    // 重置 CHANGELOG
    let changelog = doc_root_path.join("CHANGELOG.md");
    if changelog.exists() {
        fs::write(&changelog, "# Changelog\n")
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 6. git commit + tag
    let changelog_entries = read_changelog_entries(&doc_root_path);
    let commit_msg = format_commit_message(&version, &args.topic, &args.r#type, &changelog_entries);
    git_commit(&commit_msg)?;
    git_tag(&version)?;

    // 7. bump VERSION + 重置 phase
    write_version(&new_version)?;
    let next_ph = next_phase(&effective_mode, &mode);

    // 恢复 audit 模式
    if mode.starts_with("audit/") {
        yaml::set(&status_file, "mode", &effective_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    yaml::set(&status_file, "phase", &next_ph)
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    yaml::touch_updated(&status_file)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    let result = IterateOutput {
        released_version: version.clone(),
        tag: format!("v{}", version),
        archive_dir,
        archived_files,
        next_version: new_version,
        next_phase: next_ph,
    };

    if human {
        println!("[dev-flow] 迭代完成");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        println!("交付版本：v{} (tagged)", version);
        println!("新版本：v{}", result.next_version);
        println!("阶段重置：{}", result.next_phase);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn count_tasks(doc_root: &Path) -> (u32, u32) {
    let task_dir = doc_root.join("task");
    let mut total = 0u32;
    let mut done = 0u32;

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (!name.starts_with("task_") && !name.starts_with("done_task_")) || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                total += content.lines().filter(|l| l.starts_with("- [")).count() as u32;
                done += content.lines().filter(|l| l.starts_with("- [x]")).count() as u32;
            }
        }
    }
    (total, done)
}

fn count_p0_issues(doc_root: &Path) -> u32 {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return 0;
    }

    let mut p0_open = 0u32;
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("issue_") || !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let mut in_open = false;
                for line in content.lines() {
                    if line.starts_with("- [ ]") {
                        in_open = true;
                    } else if line.starts_with("- [x]") {
                        in_open = false;
                    } else if in_open && line.contains("severity:") && line.contains("P0") {
                        p0_open += 1;
                        in_open = false;
                    }
                }
            }
        }
    }
    p0_open
}

fn read_version() -> Result<String, DowError> {
    fs::read_to_string("VERSION")
        .map(|s| s.trim().to_string())
        .map_err(|_| DowError::new("VERSION 文件不存在或不可读", 1))
}

fn write_version(version: &str) -> Result<(), DowError> {
    fs::write("VERSION", format!("{}\n", version))
        .map_err(|e| DowError::new(e.to_string(), 1))
}

fn bump_version(version: &str, bump_type: &str) -> Result<String, DowError> {
    let parts: Vec<u32> = version
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .collect();

    if parts.len() != 3 {
        return Err(DowError::new(format!("版本格式非法：{}", version), 1));
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    let new = match bump_type {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        "patch" => format!("{}.{}.{}", major, minor, patch + 1),
        _ => return Err(DowError::new(format!("未知 bump 类型：{}", bump_type), 1)),
    };
    Ok(new)
}

fn next_phase(effective_mode: &str, _full_mode: &str) -> String {
    match effective_mode {
        "full" => "PRD",
        "quick" | "mvp" => "SPEC",
        "fast" => "TASK",
        _ => "DEV",
    }
    .to_string()
}

fn list_archive_files(doc_root: &Path) -> Vec<String> {
    let mut files = Vec::new();

    let task_dir = doc_root.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                files.push(name);
            }
        }
    }

    let issue_dir = doc_root.join("issue");
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("closed_issue_") && name.ends_with(".md") {
                files.push(name);
            }
        }
    }

    for doc in &["PRD.md", "SPEC.md", "TEST.md", "CHANGELOG.md"] {
        if doc_root.join(doc).exists() {
            files.push(doc.to_string());
        }
    }

    files
}

fn move_archive_files(doc_root: &Path, archive_dir: &Path) -> Result<(), DowError> {
    let task_dir = doc_root.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && (name.starts_with("done_task_") || name.starts_with("task_")) {
                fs::rename(entry.path(), archive_dir.join(&name)).ok();
            }
        }
    }

    let issue_dir = doc_root.join("issue");
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("closed_issue_") && name.ends_with(".md") {
                fs::rename(entry.path(), archive_dir.join("issue").join(&name)).ok();
            }
        }
    }

    for doc in &["PRD.md", "SPEC.md", "TEST.md"] {
        let src = doc_root.join(doc);
        if src.exists() {
            fs::rename(&src, archive_dir.join(doc)).ok();
        }
    }

    // CHANGELOG 移动后重置
    let changelog_src = doc_root.join("CHANGELOG.md");
    if changelog_src.exists() {
        fs::copy(&changelog_src, archive_dir.join("CHANGELOG.md")).ok();
    }

    Ok(())
}

fn git_commit(message: &str) -> Result<(), DowError> {
    // 只 add 需要的内容，排除 dow/target/
    Command::new("git").args(["add", "dev-doc/", "VERSION", "CLAUDE.md", "AGENTS.md"]).output().ok();
    Command::new("git").args(["add", "dow/src/", "dow/Cargo.toml", "dow/Cargo.lock", "dow/build.sh"]).output().ok();
    Command::new("git").args(["add", "scripts/", "hooks/", "tests/", ".gitignore"]).output().ok();
    Command::new("git").args(["add", ".github/"]).output().ok();
    Command::new("git").args(["add", "skills/", ".agents/", ".claude/"]).output().ok();

    let diff = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .output();

    if let Ok(d) = diff {
        if d.status.success() {
            return Ok(()); // 无变更
        }
    }

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DowError::new(format!("git commit 失败：{}", stderr), 1));
    }
    Ok(())
}

fn git_tag(version: &str) -> Result<(), DowError> {
    let tag = format!("v{}", version);

    // 检查 tag 是否已存在
    let check = Command::new("git")
        .args(["tag", "-l", &tag])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !String::from_utf8_lossy(&check.stdout).trim().is_empty() {
        return Ok(()); // tag 已存在
    }

    let output = Command::new("git")
        .args(["tag", "-a", &tag, "-m", &format!("Release {}", tag)])
        .output()
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DowError::new(format!("git tag 失败：{}", stderr), 1));
    }
    Ok(())
}

fn read_changelog_entries(doc_root: &Path) -> Vec<String> {
    let changelog = doc_root.join("CHANGELOG.md");
    let mut entries = Vec::new();

    if let Ok(content) = fs::read_to_string(&changelog) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                entries.push(trimmed.to_string());
            }
        }
    }
    entries
}

fn format_commit_message(version: &str, topic: &str, commit_type: &str, changelog: &[String]) -> String {
    let mut msg = format!("{}: Release v{} {}", commit_type, version, topic);
    if !changelog.is_empty() {
        msg.push_str("\n\n");
        for entry in changelog {
            msg.push_str(entry);
            msg.push('\n');
        }
    }
    msg
}

fn print_human_preview(result: &IterateOutput) {
    println!("[dev-flow] 迭代预览");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("当前版本：v{}", result.released_version);
    println!("归档目录：{}", result.archive_dir);
    println!("归档文件（{}个）：", result.archived_files.len());
    for f in &result.archived_files {
        println!("  - {}", f);
    }
    println!();
    println!("将要执行：");
    println!("  - git commit + tag: v{}", result.released_version);
    println!("  - bump: v{} → v{}", result.released_version, result.next_version);
    println!("  - 阶段重置：{}", result.next_phase);
}
