// dow/src/commands/
// ├── check.rs  -- dow check（文档规范检查）

use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct CheckOutput {
    pass: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    ok: Vec<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve("dev-doc");
    let status_file = doc_root_path.join("STATUS.yaml");

    if !status_file.exists() {
        return Err(DowError::new("STATUS.yaml 不存在", 1));
    }

    let map = yaml::read(&status_file).map_err(|e| DowError::new(e.to_string(), 1))?;
    let phase = map.get("phase").cloned().unwrap_or_default();
    let mode = map.get("mode").cloned().unwrap_or_default();

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ok = Vec::new();

    // 1. CHANGELOG 检查
    check_changelog(&doc_root_path, &mut warnings, &mut ok);

    // 2. task 完成度与 phase 匹配
    let (total, done) = check_tasks(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);

    // 3. issue 状态检查
    check_issues(&doc_root_path, &phase, &mut errors, &mut warnings, &mut ok);

    // 4. 代码变更 vs 文档更新时间
    check_time_sync(&status_file, &map, &mut warnings, &mut ok);

    // 5. 阶段必要文件
    check_phase_files(&doc_root_path, &phase, &mut warnings);

    // 6. SPEC 验收检查
    check_spec_ac(&doc_root_path, &mode, &mut errors, &mut warnings);

    // 7. TEST 报告检查
    check_test_report(&doc_root_path, total, done, &mut warnings);

    // 8. task nums 一致性
    check_task_nums(&doc_root_path, &mut errors);

    let pass = errors.is_empty();

    let result = CheckOutput { pass, errors, warnings, ok };

    if human {
        print_human(&result, &phase);
    } else {
        output::print_json(&result);
    }

    Ok(if pass { 0 } else { 1 })
}

fn check_changelog(doc_root: &Path, warnings: &mut Vec<String>, ok: &mut Vec<String>) {
    let changelog = doc_root.join("CHANGELOG.md");
    if changelog.exists() {
        if fs::metadata(&changelog).map(|m| m.len() > 0).unwrap_or(false) {
            ok.push("CHANGELOG.md 存在且非空".to_string());
        } else {
            warnings.push("CHANGELOG.md 为空".to_string());
        }
    } else {
        warnings.push("CHANGELOG.md 不存在".to_string());
    }
}

fn check_tasks(
    doc_root: &Path,
    phase: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    _ok: &mut Vec<String>,
) -> (usize, usize) {
    let task_dir = doc_root.join("task");
    let mut total = 0usize;
    let mut done = 0usize;

    if !task_dir.is_dir() {
        if matches!(phase, "DEV" | "TEST" | "DONE") {
            warnings.push(format!("阶段为 {} 但 task/ 目录无任务", phase));
        }
        return (0, 0);
    }

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if !name.starts_with("task_") && !name.starts_with("done_task_") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                total += content.lines().filter(|l| l.starts_with("- [")).count();
                done += content.lines().filter(|l| l.starts_with("- [x]")).count();
            }
        }
    }

    if phase == "DEV" && total == 0 {
        warnings.push("阶段为 DEV 但 task/ 目录无任务文件".to_string());
    }
    if total > 0 && done == total && phase == "DEV" {
        warnings.push("所有任务已完成但阶段仍为 DEV，建议执行 /test".to_string());
    }

    // task nums 声明 vs 实际
    let _ = errors; // nums 检查在独立函数中

    (total, done)
}

fn check_task_nums(doc_root: &Path, errors: &mut Vec<String>) {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let actual = content.lines().filter(|l| l.starts_with("- [")).count();
                // 查找 nums: 声明
                for line in content.lines() {
                    if line.starts_with("nums:") {
                        if let Some(val) = line.strip_prefix("nums:") {
                            if let Ok(declared) = val.trim().parse::<usize>() {
                                if declared != actual {
                                    errors.push(format!(
                                        "task_nums_mismatch：{} 声明 nums={}，实际任务数={}",
                                        name, declared, actual
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn check_issues(
    doc_root: &Path,
    phase: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return;
    }

    let mut open_issues = 0usize;
    let mut open_p0 = 0usize;

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
                        open_issues += 1;
                    } else if line.starts_with("- [x]") {
                        in_open = false;
                    } else if in_open && line.contains("severity:") && line.contains("P0") {
                        open_p0 += 1;
                        in_open = false;
                    }
                }
            }
        }
    }

    if open_p0 > 0 {
        errors.push(format!("open_p0_issue：有 {} 个未关闭 P0 issue", open_p0));
    }
    if open_issues > 0 && phase == "DONE" {
        warnings.push(format!("阶段为 DONE 但仍有 {} 个未关闭 issue", open_issues));
    }
    if open_issues == 0 && issue_dir.exists() {
        ok.push("所有 issue 已关闭".to_string());
    }
}

fn check_time_sync(
    _status_file: &Path,
    map: &std::collections::BTreeMap<String, String>,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let updated = map.get("updated").cloned().unwrap_or_default();
    let status_date = updated.split(' ').next().unwrap_or("");

    let commit_date = Command::new("git")
        .args(["log", "-1", "--format=%ai"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .split(' ')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            }
        });

    if let Some(commit_date) = commit_date {
        if !commit_date.is_empty() && !status_date.is_empty() && commit_date.as_str() > status_date {
            warnings.push(format!(
                "最近代码提交({})晚于 STATUS 更新({})，文档可能未同步",
                commit_date, status_date
            ));
        } else {
            ok.push("STATUS 更新时间与代码同步".to_string());
        }
    }
}

fn check_phase_files(doc_root: &Path, phase: &str, warnings: &mut Vec<String>) {
    if matches!(phase, "SPEC" | "TASK" | "DEV" | "TEST" | "DONE") {
        if !doc_root.join("SPEC.md").exists() {
            warnings.push(format!("阶段为 {} 但缺少 SPEC.md", phase));
        }
    }
}

fn check_spec_ac(doc_root: &Path, mode: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let spec_file = doc_root.join("SPEC.md");
    if !spec_file.exists() {
        return;
    }
    if let Ok(content) = fs::read_to_string(&spec_file) {
        let has_ac = content.contains("SPEC-AC-")
            || content.contains("## Acceptance")
            || content.contains("## 5. 验收契约")
            || content.contains("验收");
        if !has_ac {
            match mode {
                "full" | "quick" => {
                    errors.push(format!("spec_missing_ac：{} 模式 SPEC 缺少可测验收", mode));
                }
                _ => {
                    warnings.push("SPEC 缺少明确验收，建议补充".to_string());
                }
            }
        }
    }
}

fn check_test_report(doc_root: &Path, total: usize, done: usize, warnings: &mut Vec<String>) {
    if total > 0 && done == total {
        let test_file = doc_root.join("TEST.md");
        if !test_file.exists() {
            warnings.push("所有任务已完成但缺少 TEST.md".to_string());
        } else if let Ok(content) = fs::read_to_string(&test_file) {
            let has_failure = content.contains("FAILED SUITES:")
                || content.contains("FAIL: ")
                || content.contains("失败:")
                || content.contains("失败：")
                || content.contains("未通过：")
                || content.contains("未通过:");
            if has_failure {
                warnings.push("TEST.md 记录了未通过测试".to_string());
            }
        }
    }
}

fn print_human(result: &CheckOutput, phase: &str) {
    println!("[dev-flow] 文档同步检查");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    println!("当前阶段：{}", phase);
    println!();

    if !result.warnings.is_empty() {
        println!("⚠ 需要关注（{}项）：", result.warnings.len());
        for w in &result.warnings {
            println!("  - {}", w);
        }
        println!();
    }

    if !result.errors.is_empty() {
        println!("✗ 阻断错误（{}项）：", result.errors.len());
        for e in &result.errors {
            println!("  - {}", e);
        }
        println!();
    }

    if !result.ok.is_empty() {
        println!("✓ 正常（{}项）：", result.ok.len());
        for o in &result.ok {
            println!("  - {}", o);
        }
    }

    if result.warnings.is_empty() && result.errors.is_empty() {
        println!("✓ 文档同步状态良好，无需操作。");
    }
}
