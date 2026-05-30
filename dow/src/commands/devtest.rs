// dow/src/commands/
// ├── devtest.rs  -- dow devtest（任务级测试）

use crate::cli::DevtestArgs;
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct DevtestOutput {
    task: String,
    result: String,
    tests_run: Vec<String>,
    total: u32,
    passed: u32,
    failed: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failures: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_unchecked: Option<bool>,
}

pub fn run(args: DevtestArgs, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let task_dir = doc_root_path.join("task");

    if !task_dir.is_dir() {
        return Err(DowError::new("task/ 目录不存在", 1));
    }

    // 找到目标任务
    let (task_file, task_line, task_title, test_files) = find_target_task(&task_dir, args.task.as_deref())?;

    if test_files.is_empty() {
        let result = DevtestOutput {
            task: task_title,
            result: "NEEDS_CONTEXT".to_string(),
            tests_run: vec![],
            total: 0,
            passed: 0,
            failed: 0,
            failures: vec![],
            issue_created: None,
            task_unchecked: None,
        };
        if human {
            println!("[dev-flow] devtest NEEDS_CONTEXT：无测试文件");
        } else {
            output::print_json(&result);
        }
        return Ok(2);
    }

    // 运行测试
    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failures = Vec::new();

    for test_file in &test_files {
        total += 1;
        if !Path::new(test_file).exists() {
            failed += 1;
            failures.push(format!("{}: 文件不存在", test_file));
            continue;
        }
        let output = Command::new("bash").arg(test_file).output();
        match output {
            Ok(r) => {
                if r.status.success() {
                    passed += 1;
                } else {
                    failed += 1;
                    let stderr = String::from_utf8_lossy(&r.stderr).trim().to_string();
                    failures.push(format!("{}: {}", test_file, stderr.lines().next().unwrap_or("exit non-zero")));
                }
            }
            Err(e) => {
                failed += 1;
                failures.push(format!("{}: {}", test_file, e));
            }
        }
    }

    if failed == 0 {
        let result = DevtestOutput {
            task: task_title.clone(),
            result: "PASS".to_string(),
            tests_run: test_files,
            total,
            passed,
            failed: 0,
            failures: vec![],
            issue_created: None,
            task_unchecked: None,
        };
        if human {
            println!("[dev-flow] devtest PASS：{}", task_title);
        } else {
            output::print_json(&result);
        }
        Ok(0)
    } else {
        // FAIL：取消勾选 + 创建 issue
        uncheck_task(&task_file, task_line);
        let issue_path = create_devtest_issue(&doc_root_path, &task_title, &failures);

        let result = DevtestOutput {
            task: task_title.clone(),
            result: "FAIL".to_string(),
            tests_run: test_files,
            total,
            passed,
            failed,
            failures,
            issue_created: Some(issue_path),
            task_unchecked: Some(true),
        };
        if human {
            println!("[dev-flow] devtest FAIL：{}", task_title);
            if let Some(ref issue) = result.issue_created {
                println!("→ 已创建 issue：{}", issue);
            }
        } else {
            output::print_json(&result);
        }
        Ok(1)
    }
}

fn find_target_task(
    task_dir: &Path,
    target_id: Option<&str>,
) -> Result<(String, usize, String, Vec<String>), DowError> {
    let mut entries: Vec<_> = fs::read_dir(task_dir)
        .map_err(|e| DowError::new(e.to_string(), 1))?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("task_") && n.ends_with(".md")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries.iter().rev() {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate().rev() {
                if line.starts_with("- [x]") {
                    let title = line.trim_start_matches("- [x] ").to_string();

                    // 如果指定了 task id，检查是否匹配
                    if let Some(id) = target_id {
                        if !title.contains(id) {
                            continue;
                        }
                    }

                    // 查找 files.test 字段
                    let test_files = extract_test_files(&lines, i);

                    return Ok((
                        entry.path().to_string_lossy().to_string(),
                        i + 1, // 1-based
                        title,
                        test_files,
                    ));
                }
            }
        }
    }

    Err(DowError::new("未找到已完成的任务", 1))
}

fn extract_test_files(lines: &[&str], task_line: usize) -> Vec<String> {
    let mut in_test = false;
    let mut tests = Vec::new();

    for line in lines.iter().skip(task_line + 1) {
        // 下一个任务条目开始
        if line.starts_with("- [") {
            break;
        }
        if line.contains("test:") {
            in_test = true;
            // 内联格式: test: ["tests/xxx.sh"]
            if let Some(bracket_start) = line.find('[') {
                if let Some(bracket_end) = line.find(']') {
                    let inner = &line[bracket_start + 1..bracket_end];
                    for item in inner.split(',') {
                        let cleaned = item.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !cleaned.is_empty() {
                            tests.push(cleaned);
                        }
                    }
                    in_test = false;
                }
            }
        } else if in_test {
            // 多行列表格式
            let trimmed = line.trim();
            if trimmed.starts_with('-') || trimmed.starts_with("\"") {
                let cleaned = trimmed
                    .trim_start_matches('-')
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !cleaned.is_empty() {
                    tests.push(cleaned);
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                in_test = false;
            }
        }
    }

    tests
}

fn uncheck_task(task_file: &str, line_num: usize) {
    if let Ok(content) = fs::read_to_string(task_file) {
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines: Vec<String> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i + 1 == line_num && line.starts_with("- [x]") {
                new_lines.push(line.replacen("- [x]", "- [ ]", 1));
            } else {
                new_lines.push(line.to_string());
            }
        }
        let mut output = new_lines.join("\n");
        if !output.ends_with('\n') {
            output.push('\n');
        }
        fs::write(task_file, output).ok();
    }
}

fn create_devtest_issue(doc_root: &Path, task_title: &str, failures: &[String]) -> String {
    let issue_dir = doc_root.join("issue");
    fs::create_dir_all(&issue_dir).ok();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut seq = 1u32;
    while issue_dir
        .join(format!("issue_devtest_{}_{}.md", today, seq))
        .exists()
    {
        seq += 1;
    }

    let filename = format!("issue_devtest_{}_{}.md", today, seq);
    let path = issue_dir.join(&filename);

    let detail = failures.join("; ");
    let content = format!(
        "---\nsource: devtest\nnums: 1\n---\n\n- [ ] ISSUE-I001: devtest 未通过：{}\n  - severity: P1\n  - source: devtest\n  - location: {}\n  - current: {}\n  - expected: task 通过 devtest\n  - reproduce: dow devtest\n  - root_cause:\n  - fix:\n  - close_when: 重新执行 devtest 返回 PASS\n",
        task_title, task_title, detail
    );

    fs::write(&path, content).ok();
    path.to_string_lossy().to_string()
}
