// dow/src/commands/
// ├── iterate.rs  -- dow iterate（迭代交付：校验 → 归档 → commit + tag → bump）
//
// Related Docs:
// - [CLAUDE.md - 命令](../../../CLAUDE.md#命令)
// - [dev-flow 规范](../../references/dev-flow-spec.md)

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
    pre_iterate: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    commit_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

#[derive(Clone, Debug)]
enum PreIterateStep {
    SyncVersion { path: String },
    Run { name: String, command: String },
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

    // 2.6 检查持久化文档同步（警告但不阻断）
    let doc_warnings = check_persistent_docs_sync(&status_file);
    if !doc_warnings.is_empty() && !args.confirm {
        if human {
            println!("[dev-flow] 警告：以下持久化文档自上次迭代后未更新：");
            for w in &doc_warnings {
                println!("  - {}", w);
            }
            println!();
        }
    }

    // 3. 计算版本：当前版本即 released_version，bump 一次得到 next_version
    let released_version = version::read_current()?;
    let next_version = version::bump_version_str(&released_version, &args.bump)?;

    // 4. 计算归档内容
    let archive_base = archive_db::archive_base();
    let archive_db_path = format!("{}/archive.db", archive_base.to_string_lossy());
    let archived_files = list_archive_files(&doc_root_path);

    // --confirm 模式：验证 token 后执行
    if args.confirm {
        let tokens = generate_tokens_with_window(&args);
        let found = tokens.iter().any(|t| {
            let env_key = format!("DOW_ITERATE_{}", t);
            std::env::var(&env_key).is_ok()
        });
        if !found {
            let hint = &tokens[0];
            return Err(DowError::new(
                format!(
                    "确认失败：环境变量 DOW_ITERATE_{} 不存在，请先执行 dow iterate 预览",
                    hint
                ),
                1,
            ));
        }
    } else {
        // 预览前先触发 save_changelog，确保当前会话活动被记录
        if let Err(e) = crate::hooks::save_changelog::run(false, false) {
            eprintln!("[dev-flow] save_changelog 警告：{}", e.message);
        }

        // 生成 token（预览模式）
        let token = generate_token_for_minute(0, &args);
        // 默认：输出预览
        let commit_files = list_pending_changes(&args.files);
        let should_tag = args.bump != "patch" || args.tag;
        let changelog_entries = read_changelog_entries(&doc_root_path);
        let pre_iterate = describe_pre_iterate_steps()?;
        let result = IterateOutput {
            tag: if should_tag {
                format!("v{}", &released_version)
            } else {
                "no-tag".to_string()
            },
            released_version: released_version.clone(),
            archive_db: archive_db_path.clone(),
            archived_files: archived_files.clone(),
            next_version: next_version.clone(),
            next_phase: next_phase(&effective_mode, &mode),
            pre_iterate,
            commit_files,
            token: Some(token),
        };
        if human {
            print_human_preview(&result);
            print_changelog_summary(&changelog_entries);
        } else {
            let mut json_out = serde_json::to_value(&result).unwrap_or_default();
            json_out["changelog_entries"] = serde_json::json!(changelog_entries);
            json_out["changelog_hint"] = serde_json::json!(
                "请检查 CHANGELOG 是否有遗漏的记录。如有遗漏，请在确认前手动补充。"
            );
            if !doc_warnings.is_empty() {
                json_out["doc_sync_warnings"] = serde_json::json!(doc_warnings);
            }
            println!("{}", serde_json::to_string_pretty(&json_out).unwrap());
        }
        return Ok(0);
    }

    // 5. 读取 CHANGELOG（归档前，保留 commit body 内容）
    let changelog_entries = read_changelog_entries(&doc_root_path);

    // 5.5 preIterate CI：必须在归档、commit、tag、bump 之前执行，失败即阻断整个 iterate
    let pre_iterate = run_pre_iterate(&released_version, human)?;

    // 6. bump VERSION 先写入（归档版本）
    version::write_current(&released_version)?;

    // 7. 执行归档（写入 SQLite）
    let conn = archive_db::open_or_create(&archive_base)?;
    let released_at = chrono::Local::now().format("%Y-%m-%d").to_string();
    let cur_branch = doc_root::current_branch().unwrap_or_else(|| "main".to_string());
    archive_db::insert_iteration(
        &conn,
        &archive_db::IterationRecord {
            version: released_version.clone(),
            topic: args.topic.clone(),
            commit_type: Some(args.r#type.clone()),
            branch: cur_branch,
            released_at,
            tag: format!("v{}", released_version),
            mode: Some(effective_mode.clone()),
        },
    )?;

    // 归档 task 文件
    let task_dir = doc_root_path.join("task");
    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && (name.starts_with("done_task_") || name.starts_with("task_"))
            {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let tasks = archive_db::parse_task_file(&name, &content);
                    for task in &tasks {
                        archive_db::insert_task(&conn, &released_version, task)?;
                    }
                }
                if let Err(e) = fs::remove_file(entry.path()) {
                    eprintln!("[dev-flow] 警告：归档后删除 {} 失败: {}", name, e);
                }
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
                if let Err(e) = fs::remove_file(entry.path()) {
                    eprintln!("[dev-flow] 警告：归档后删除 {} 失败: {}", name, e);
                }
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
            if let Err(e) = fs::remove_file(&src) {
                eprintln!("[dev-flow] 警告：归档后删除 {}.md 失败: {}", doc_type, e);
            }
        }
    }

    // 归档 CHANGELOG
    let changelog = doc_root_path.join("CHANGELOG.md");
    if changelog.exists() {
        if let Ok(content) = fs::read_to_string(&changelog) {
            let cl_entries = archive_db::parse_changelog(&content);
            for (order, (date, text)) in cl_entries.iter().enumerate() {
                archive_db::insert_changelog(
                    &conn,
                    &released_version,
                    date.as_deref(),
                    text,
                    order as i32,
                )?;
            }
        }
        fs::write(&changelog, "# Changelog\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 7.5 清理 claim.lock（归档后不再需要）
    let claim_lock = doc_root_path.join("claim.lock");
    if claim_lock.exists() {
        let _ = fs::remove_file(&claim_lock);
    }

    // 8. git commit + tag（archive.db 加入提交）
    let commit_msg = format_commit_message(
        &released_version,
        &args.topic,
        &args.r#type,
        &changelog_entries,
    );
    let mut commit_files = args.files.clone();
    for file in &pre_iterate {
        if !commit_files.contains(file) {
            commit_files.push(file.clone());
        }
    }
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

    yaml::set(&status_file, "phase", &next_ph).map_err(|e| DowError::new(e.to_string(), 1))?;
    yaml::touch_updated(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;

    let tag_str = if should_tag {
        format!("v{}", released_version)
    } else {
        "no-tag".to_string()
    };
    let result = IterateOutput {
        released_version: released_version.clone(),
        tag: tag_str.clone(),
        archive_db: archive_db_path,
        archived_files,
        next_version: next_version,
        next_phase: next_ph,
        pre_iterate,
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
            if (!name.starts_with("task_") && !name.starts_with("done_task_"))
                || !name.ends_with(".md")
            {
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

fn describe_pre_iterate_steps() -> Result<Vec<String>, DowError> {
    let steps = read_pre_iterate_steps()?;
    Ok(steps.iter().map(describe_pre_iterate_step).collect())
}

fn run_pre_iterate(version: &str, human: bool) -> Result<Vec<String>, DowError> {
    let steps = read_pre_iterate_steps()?;
    if steps.is_empty() {
        return Ok(Vec::new());
    }

    let before_paths = git_worktree_paths();
    if human {
        println!("[dev-flow] 执行 preIterate steps");
    }

    let mut changed_files = Vec::new();
    for step in steps {
        match step {
            PreIterateStep::SyncVersion { path } => {
                if human {
                    println!("  - sync-version: {} -> {}", path, version);
                }
                if sync_version_file(&path, version)? && !changed_files.contains(&path) {
                    changed_files.push(path);
                }
            }
            PreIterateStep::Run { name, command } => {
                if human {
                    println!("  - {}: {}", name, command);
                }
                run_shell_step(&name, &command)?;
            }
        }
    }
    for file in git_worktree_paths() {
        if !before_paths.contains(&file) && !changed_files.contains(&file) {
            changed_files.push(file);
        }
    }
    Ok(changed_files)
}

fn read_pre_iterate_steps() -> Result<Vec<PreIterateStep>, DowError> {
    let path = Path::new(".dev-doc/preIterate.yaml");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("读取 .dev-doc/preIterate.yaml 失败：{}", e), 1))?;
    parse_pre_iterate_steps(&content)
}

fn parse_pre_iterate_steps(content: &str) -> Result<Vec<PreIterateStep>, DowError> {
    let mut steps = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "sync-version" {
            return Err(DowError::new(
                "preIterate sync-version 必须显式声明目标文件，例如 `sync-version: dow/Cargo.toml`",
                1,
            ));
        }
        if let Some(path) = trimmed.strip_prefix("sync-version:") {
            let path = unquote(path.trim());
            if path.is_empty() {
                return Err(DowError::new("preIterate sync-version 目标不能为空", 1));
            }
            steps.push(PreIterateStep::SyncVersion { path });
            continue;
        }
        if let Some(command) = trimmed.strip_prefix("run:") {
            let command = unquote(command.trim());
            if command.is_empty() {
                return Err(DowError::new("preIterate run step 不能为空", 1));
            }
            steps.push(PreIterateStep::Run {
                name: format!("run: {}", command),
                command,
            });
            continue;
        }
        return Err(DowError::new(
            format!("preIterate step 不支持：{}", trimmed),
            1,
        ));
    }
    Ok(steps)
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn describe_pre_iterate_step(step: &PreIterateStep) -> String {
    match step {
        PreIterateStep::SyncVersion { path } => format!("sync-version: {}", path),
        PreIterateStep::Run { name, command } => format!("{}: {}", name, command),
    }
}

fn run_shell_step(name: &str, command: &str) -> Result<(), DowError> {
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    }
    .map_err(|e| DowError::new(format!("preIterate step `{}` 启动失败：{}", name, e), 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        return Err(DowError::new(
            format!("preIterate step `{}` 失败：{}", name, detail),
            1,
        ));
    }
    Ok(())
}

fn sync_version_file(path: &str, version: &str) -> Result<bool, DowError> {
    let manifest = Path::new(path);
    if !manifest.exists() {
        return Err(DowError::new(
            format!("preIterate sync-version 目标不存在：{}", path),
            1,
        ));
    }
    match manifest.file_name().and_then(|n| n.to_str()) {
        Some("Cargo.toml") => update_toml_version(manifest, version, &["package"]),
        Some("package.json") => update_package_json_version(manifest, version),
        Some("pyproject.toml") => {
            let project = update_toml_version(manifest, version, &["project"])?;
            let poetry = update_toml_version(manifest, version, &["tool", "poetry"])?;
            Ok(project || poetry)
        }
        _ => Err(DowError::new(
            format!("preIterate sync-version 不支持的文件：{}", path),
            1,
        )),
    }
}

fn update_package_json_version(path: &Path, version: &str) -> Result<bool, DowError> {
    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("读取 {} 失败：{}", path.display(), e), 1))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| DowError::new(format!("解析 {} 失败：{}", path.display(), e), 1))?;
    let current = json.get("version").and_then(|v| v.as_str());
    if current == Some(version) {
        return Ok(false);
    }
    json["version"] = serde_json::Value::String(version.to_string());
    let output = serde_json::to_string_pretty(&json)
        .map_err(|e| DowError::new(format!("序列化 {} 失败：{}", path.display(), e), 1))?;
    fs::write(path, format!("{}\n", output))
        .map_err(|e| DowError::new(format!("写入 {} 失败：{}", path.display(), e), 1))?;
    Ok(true)
}

fn update_toml_version(path: &Path, version: &str, section: &[&str]) -> Result<bool, DowError> {
    let content = fs::read_to_string(path)
        .map_err(|e| DowError::new(format!("读取 {} 失败：{}", path.display(), e), 1))?;
    let mut value: toml::Value = toml::from_str(&content)
        .map_err(|e| DowError::new(format!("解析 {} 失败：{}", path.display(), e), 1))?;

    let mut table = &mut value;
    for key in section {
        match table.get_mut(*key) {
            Some(next) => table = next,
            None => return Ok(false),
        }
    }

    let current = table.get("version").and_then(|v| v.as_str());
    if current == Some(version) {
        return Ok(false);
    }
    if let Some(tbl) = table.as_table_mut() {
        tbl.insert(
            "version".to_string(),
            toml::Value::String(version.to_string()),
        );
    } else {
        return Ok(false);
    }

    let output = toml::to_string_pretty(&value)
        .map_err(|e| DowError::new(format!("序列化 {} 失败：{}", path.display(), e), 1))?;
    fs::write(path, output)
        .map_err(|e| DowError::new(format!("写入 {} 失败：{}", path.display(), e), 1))?;
    Ok(true)
}

fn git_worktree_paths() -> Vec<String> {
    let Ok(output) = Command::new("git").args(["status", "--porcelain"]).output() else {
        return Vec::new();
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let path = line[3..]
                .split(" -> ")
                .last()
                .unwrap_or("")
                .trim()
                .to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        })
        .collect()
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

    for doc in &[
        "PRD.md",
        "SPEC.md",
        "TEST.md",
        "BRAINSTORM.md",
        "CHANGELOG.md",
    ] {
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

fn format_commit_message(
    version: &str,
    topic: &str,
    commit_type: &str,
    changelog: &[String],
) -> String {
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

fn print_changelog_summary(entries: &[String]) {
    if entries.is_empty() {
        println!();
        println!("⚠ CHANGELOG 为空，请检查是否有遗漏的记录。");
        println!("  如有遗漏，请在确认前手动补充到 CHANGELOG.md。");
    } else {
        println!();
        println!("CHANGELOG 当前条目（{}条）：", entries.len());
        for entry in entries {
            println!("  {}", entry);
        }
        println!();
        println!("提示：请检查 CHANGELOG 是否有遗漏。如需补充，请在确认前编辑 CHANGELOG.md。");
    }
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
    if !result.pre_iterate.is_empty() {
        println!("preIterate steps（{}个）：", result.pre_iterate.len());
        for step in &result.pre_iterate {
            println!("  - {}", step);
        }
        println!();
    }
    println!("将要执行：");
    println!("  - git commit + tag: v{}", result.released_version);
    if !result.pre_iterate.is_empty() {
        println!("  - preIterate: git commit 前执行");
    }
    println!(
        "  - bump: v{} → v{}",
        result.released_version, result.next_version
    );
    println!("  - 阶段重置：{}", result.next_phase);
    if let Some(ref t) = result.token {
        println!();
        println!("确认执行：DOW_ITERATE_{}=1 dow iterate --confirm ...", t);
    }
}

fn generate_token_for_minute(offset: i64, args: &IterateArgs) -> String {
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
    args.topic.hash(&mut hasher);
    args.r#type.hash(&mut hasher);
    args.bump.hash(&mut hasher);
    args.files.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)[..8].to_string()
}

// 返回当前分钟 + 前4分钟的 token（5分钟有效窗口）
fn generate_tokens_with_window(args: &IterateArgs) -> Vec<String> {
    (0..=4)
        .map(|i| generate_token_for_minute(-i, args))
        .collect()
}

fn check_persistent_docs_sync(status_file: &Path) -> Vec<String> {
    let docs = yaml::get_list(status_file, "docs").unwrap_or_default();
    if docs.is_empty() {
        return Vec::new();
    }

    // 找最近的 tag 作为 --since 参考
    let last_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let git_ref = match last_tag {
        Some(ref t) if !t.is_empty() => t.as_str(),
        _ => return Vec::new(),
    };

    // 验证 ref 有效
    let ref_check = Command::new("git")
        .args(["rev-parse", "--verify", git_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !ref_check {
        return Vec::new();
    }

    let mut all_docs = docs;
    all_docs.push("README.md".to_string());

    let mut outdated = Vec::new();
    for doc in &all_docs {
        let changed = Command::new("git")
            .args(["log", &format!("{}..HEAD", git_ref), "--", doc])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        if !changed && Path::new(doc).exists() {
            outdated.push(doc.clone());
        }
    }
    outdated
}

fn list_pending_changes(extra_files: &[String]) -> Vec<String> {
    let mut changes = Vec::new();

    // 已追踪文件的工作区修改（git add -u 会提交的内容）
    if let Ok(output) = Command::new("git").args(["diff", "--name-only"]).output() {
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
