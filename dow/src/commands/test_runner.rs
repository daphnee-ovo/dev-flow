// dow/src/commands/
// ├── test_runner.rs  -- dow test (unified test runner: full suite + task-level devtest)
//
// Behavior:
//   --task <id>  → devtest mode: find task's test files, run them, uncheck on fail + create issue
//   --file <f>   → run single file
//   (neither)    → discover and run full test suite
//
// Related Docs:
// - [CLAUDE.md - dow CLI](../../../CLAUDE.md#dow-cli)

use crate::cli::TestArgs;
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

// ─── Output structs ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TestOutput {
    total: u32,
    passed: u32,
    failed: u32,
    failures: Vec<String>,
}

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

// ─── Public entry ───────────────────────────────────────────────────────────

pub fn run(args: TestArgs, human: bool) -> Result<i32, DowError> {
    if args.task.is_some() {
        run_devtest(args.task.as_deref(), human)
    } else {
        run_full_test(args.file.as_deref(), human)
    }
}

// ─── Full test suite ────────────────────────────────────────────────────────

fn run_full_test(file: Option<&str>, human: bool) -> Result<i32, DowError> {
    let test_files = if let Some(f) = file {
        vec![f.to_string()]
    } else {
        discover_tests()?
    };

    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failures = Vec::new();

    for test_file in &test_files {
        total += 1;
        let output = Command::new("bash").arg(test_file).output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    passed += 1;
                } else {
                    failed += 1;
                    let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
                    let stdout_full =
                        String::from_utf8_lossy(&result.stdout).trim().to_string();
                    let detail = if !stderr.is_empty() {
                        stderr
                    } else {
                        stdout_full
                    };
                    let last_line = detail.lines().last().unwrap_or("").to_string();
                    failures.push(format!("{}: {}", test_file, last_line));
                }
            }
            Err(e) => {
                failed += 1;
                failures.push(format!("{}: {}", test_file, e));
            }
        }
    }

    let result = TestOutput {
        total,
        passed,
        failed,
        failures,
    };

    if human {
        println!("[dev-flow] Test result: {}/{} passed", passed, total);
        if !result.failures.is_empty() {
            println!("Failed:");
            for f in &result.failures {
                println!("  - {}", f);
            }
        }
    } else {
        output::print_json(&result);
    }

    Ok(if failed > 0 { 1 } else { 0 })
}

fn discover_tests() -> Result<Vec<String>, DowError> {
    let mut tests = Vec::new();

    if let Ok(entries) = fs::read_dir("tests") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("test_") && name.ends_with(".sh") && name != "test_all.sh" {
                tests.push(format!("tests/{}", name));
            }
        }
    }

    tests.sort();
    Ok(tests)
}

// ─── Devtest (task-level) ───────────────────────────────────────────────────

fn run_devtest(target_id: Option<&str>, human: bool) -> Result<i32, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let task_dir = doc_root_path.join("task");

    if !task_dir.is_dir() {
        return Err(DowError::new("task/ directory does not exist", 1));
    }

    // Find target task
    let (task_file, task_line, task_title, test_files) =
        find_target_task(&task_dir, target_id)?;

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
            println!("[dev-flow] devtest NEEDS_CONTEXT: no test files");
        } else {
            output::print_json(&result);
        }
        return Ok(2);
    }

    // Run tests
    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failures = Vec::new();

    for test_file in &test_files {
        total += 1;
        if !Path::new(test_file).exists() {
            failed += 1;
            failures.push(format!("{}: file does not exist", test_file));
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
                    failures.push(format!(
                        "{}: {}",
                        test_file,
                        stderr.lines().next().unwrap_or("exit non-zero")
                    ));
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
            println!("[dev-flow] devtest PASS: {}", task_title);
        } else {
            output::print_json(&result);
        }
        Ok(0)
    } else {
        // FAIL: uncheck task + create issue
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
            println!("[dev-flow] devtest FAIL: {}", task_title);
            if let Some(ref issue) = result.issue_created {
                println!("→ Issue created: {}", issue);
            }
        } else {
            output::print_json(&result);
        }
        Ok(1)
    }
}

// ─── Devtest helpers ────────────────────────────────────────────────────────

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

                    // If task id is specified, check if it matches
                    if let Some(id) = target_id {
                        if !title.contains(id) {
                            continue;
                        }
                    }

                    // Find files.test field
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

    Err(DowError::new("No completed task found", 1))
}

fn extract_test_files(lines: &[&str], task_line: usize) -> Vec<String> {
    let mut in_test = false;
    let mut tests = Vec::new();

    for line in lines.iter().skip(task_line + 1) {
        // Next task entry starts
        if line.starts_with("- [") {
            break;
        }
        if line.contains("test:") {
            in_test = true;
            // Inline format: test: ["tests/xxx.sh"]
            if let Some(bracket_start) = line.find('[') {
                if let Some(bracket_end) = line.find(']') {
                    let inner = &line[bracket_start + 1..bracket_end];
                    for item in inner.split(',') {
                        let cleaned =
                            item.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !cleaned.is_empty() {
                            tests.push(cleaned);
                        }
                    }
                    in_test = false;
                }
            }
        } else if in_test {
            // Multi-line list format
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
        let mut out = new_lines.join("\n");
        if !out.ends_with('\n') {
            out.push('\n');
        }
        fs::write(task_file, out).ok();
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
        "---\nsource: devtest\nnums: 1\n---\n\n- [ ] ISSUE-I001: devtest failed: {}\n  - severity: P1\n  - source: devtest\n  - location: {}\n  - current: {}\n  - expected: task passes devtest\n  - reproduce: dow test --task\n  - root_cause:\n  - fix:\n  - close_when: Re-running devtest returns PASS\n",
        task_title, task_title, detail
    );

    fs::write(&path, content).ok();
    path.to_string_lossy().to_string()
}
