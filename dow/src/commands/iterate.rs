// dow/src/commands/
// ├── iterate.rs  -- dow iterate（迭代交付：校验 → 归档 → commit + tag → bump）

use crate::cli::IterateArgs;
use crate::core::{archive_db, doc_root, doc_validator, version, yaml};
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
    archive_db: String,
    archived_files: Vec<String>,
    next_version: String,
    next_phase: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    commit_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

pub fn run(args: IterateArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
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

    // 2.5 校验：所有 .dev-doc 文件合法
    let validation_errors = doc_validator::validate_all(&doc_root_path);
    if !validation_errors.is_empty() {
        let msg = format!(
            "iterate 前置检查失败：.dev-doc 文件存在格式错误。\n{}",
            doc_validator::format_errors_human(&validation_errors)
        );
        return Err(DowError::new(msg, 1));
    }

    // 3. 计算版本：当前 → bump → released_version（归档版本），再 +patch = next_version
    let cur_version = version::read_current()?;
    let released_version = version::bump_version_str(&cur_version, &args.bump)?;
    let next_version = version::bump_version_str(&released_version, "patch")?;

    // 4. 计算归档内容
    let archive_base = archive_db::archive_base();
    let archive_db_path = format!("{}/archive.db", archive_base.to_string_lossy());
    let archived_files = list_archive_files(&doc_root_path);

    // --confirm 模式：验证 token 后执行
    if args.confirm {
        let tokens = generate_tokens_with_window();
        let found = tokens.iter().any(|t| {
            let env_key = format!("DOW_ITERATE_{}", t);
            std::env::var(&env_key).is_ok()
        });
        if !found {
            let hint = &tokens[0];
            return Err(DowError::new(
                format!("确认失败：环境变量 DOW_ITERATE_{} 不存在，请先执行 dow iterate 预览", hint),
                1,
            ));
        }
    } else {
        // 生成 token（预览模式）
        let token = generate_token_for_minute(0);
        // 默认：输出预览
        let commit_files = list_pending_changes(&args.files);
        let should_tag = args.bump != "patch" || args.tag;
        let result = IterateOutput {
            tag: if should_tag { format!("v{}", &released_version) } else { "no-tag".to_string() },
            released_version: released_version.clone(),
            archive_db: archive_db_path.clone(),
            archived_files: archived_files.clone(),
            next_version: next_version.clone(),
            next_phase: next_phase(&effective_mode, &mode),
            commit_files,
            token: Some(token),
        };
        if human {
            print_human_preview(&result);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    // 5. 读取 CHANGELOG（归档前，保留 commit body 内容）
    let changelog_entries = read_changelog_entries(&doc_root_path);

    // 6. bump VERSION 先写入（归档版本）
    version::write_current(&released_version)?;

    // 7. 执行归档（写入 SQLite）
    let conn = archive_db::open_or_create(&archive_base)?;
    let released_at = chrono::Local::now().format("%Y-%m-%d").to_string();
    let cur_branch = doc_root::current_branch().unwrap_or_else(|| "main".to_string());
    archive_db::insert_iteration(&conn, &archive_db::IterationRecord {
        version: released_version.clone(),
        topic: args.topic.clone(),
        commit_type: Some(args.r#type.clone()),
        branch: cur_branch,
        released_at,
        tag: format!("v{}", released_version),
        mode: Some(effective_mode.clone()),
    })?;

    // 归档 task 文件
    let task_dir = doc_root_path.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && (name.starts_with("done_task_") || name.starts_with("task_")) {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let tasks = archive_db::parse_task_file(&name, &content);
                    for task in &tasks {
                        archive_db::insert_task(&conn, &released_version, task)?;
                    }
                }
                fs::remove_file(entry.path()).ok();
            }
        }
    }

    // 归档 closed issue
    let issue_dir = doc_root_path.join("issue");
    if let Ok(entries) = fs::read_dir(&issue_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("closed_issue_") && name.ends_with(".md") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let issues = archive_db::parse_issue_file(&name, &content);
                    for issue in &issues {
                        archive_db::insert_issue(&conn, &released_version, issue)?;
                    }
                }
                fs::remove_file(entry.path()).ok();
            }
        }
    }

    // 归档文档（PRD/SPEC/TEST/BRAINSTORM）
    for doc_type in &["PRD", "SPEC", "TEST", "BRAINSTORM"] {
        let src = doc_root_path.join(format!("{}.md", doc_type));
        if src.exists() {
            if let Ok(content) = fs::read_to_string(&src) {
                archive_db::insert_doc(&conn, &released_version, doc_type, &content)?;
            }
            fs::remove_file(&src).ok();
        }
    }

    // 归档 CHANGELOG
    let changelog = doc_root_path.join("CHANGELOG.md");
    if changelog.exists() {
        if let Ok(content) = fs::read_to_string(&changelog) {
            let cl_entries = archive_db::parse_changelog(&content);
            for (order, (date, text)) in cl_entries.iter().enumerate() {
                archive_db::insert_changelog(&conn, &released_version, date.as_deref(), text, order as i32)?;
            }
        }
        fs::write(&changelog, "# Changelog\n")
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 8. git commit + tag（archive.db 加入提交）
    let commit_msg = format_commit_message(&released_version, &args.topic, &args.r#type, &changelog_entries);
    let mut commit_files = args.files.clone();
    commit_files.push(archive_db_path.clone());
    git_commit(&commit_msg, &commit_files)?;

    // 只有 minor/major 或显式 --tag 才打 git tag
    let should_tag = args.bump != "patch" || args.tag;
    if should_tag {
        git_tag(&released_version)?;
    }

    // 9. bump VERSION 到 next_version + 重置 phase
    version::write_current(&next_version)?;
    let next_ph = next_phase(&effective_mode, &mode);

    if mode.starts_with("audit/") {
        yaml::set(&status_file, "mode", &effective_mode)
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    yaml::set(&status_file, "phase", &next_ph)
        .map_err(|e| DowError::new(e.to_string(), 1))?;
    yaml::touch_updated(&status_file)
        .map_err(|e| DowError::new(e.to_string(), 1))?;


    let tag_str = if should_tag { format!("v{}", released_version) } else { "no-tag".to_string() };
    let result = IterateOutput {
        released_version: released_version.clone(),
        tag: tag_str.clone(),
        archive_db: archive_db_path,
        archived_files,
        next_version: next_version,
        next_phase: next_ph,
        commit_files: vec![],
        token: None,
    };

    if human {
        println!("[dev-flow] 迭代完成");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        let tag_display = if should_tag { " (tagged)" } else { "" };
        println!("交付版本：v{}{}", released_version, tag_display);
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

    for doc in &["PRD.md", "SPEC.md", "TEST.md", "BRAINSTORM.md", "CHANGELOG.md"] {
        if doc_root.join(doc).exists() {
            files.push(doc.to_string());
        }
    }

    files
}


fn git_commit(message: &str, extra_files: &[String]) -> Result<(), DowError> {
    // 已追踪文件的修改/删除
    Command::new("git").args(["add", "-u"]).output().ok();
    // 额外指定的新文件/目录
    for f in extra_files {
        Command::new("git").args(["add", f]).output().ok();
    }

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
    println!("归档数据库：{}", result.archive_db);
    println!("归档文件（{}个）：", result.archived_files.len());
    for f in &result.archived_files {
        println!("  - {}", f);
    }
    println!();
    if !result.commit_files.is_empty() {
        println!("提交文件（{}个）：", result.commit_files.len());
        for f in &result.commit_files {
            println!("  - {}", f);
        }
        println!();
    }
    println!("将要执行：");
    println!("  - git commit + tag: v{}", result.released_version);
    println!("  - bump: v{} → v{}", result.released_version, result.next_version);
    println!("  - 阶段重置：{}", result.next_phase);
    if let Some(ref t) = result.token {
        println!();
        println!("确认执行：DOW_ITERATE_{}=1 dow iterate --confirm ...", t);
    }
}

fn generate_token_for_minute(offset: i64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let now = chrono::Local::now() + chrono::Duration::minutes(offset);
    let minute_key = now.format("%Y-%m-%d-%H-%M").to_string();

    let mut hasher = DefaultHasher::new();
    cwd.hash(&mut hasher);
    minute_key.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)[..8].to_string()
}

// 返回当前分钟 + 前4分钟的 token（5分钟有效窗口）
fn generate_tokens_with_window() -> Vec<String> {
    (0..=4).map(|i| generate_token_for_minute(-i)).collect()
}

fn list_pending_changes(extra_files: &[String]) -> Vec<String> {
    let mut changes = Vec::new();

    // 已追踪文件的工作区修改（git add -u 会提交的内容）
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.is_empty() {
                changes.push(line.to_string());
            }
        }
    }

    // 已 staged 的变更
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.is_empty() && !changes.contains(&line.to_string()) {
                changes.push(line.to_string());
            }
        }
    }

    // 额外指定的文件
    for f in extra_files {
        if !changes.contains(f) {
            changes.push(f.clone());
        }
    }

    changes
}
