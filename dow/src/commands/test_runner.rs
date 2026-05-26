// dow/src/commands/
// ├── test_runner.rs  -- dow test（全量测试运行器）

use crate::cli::TestArgs;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Serialize)]
struct TestOutput {
    total: u32,
    passed: u32,
    failed: u32,
    failures: Vec<String>,
}

pub fn run(args: TestArgs, human: bool) -> Result<i32, DowError> {
    let test_files = if let Some(ref file) = args.file {
        vec![file.clone()]
    } else {
        discover_tests()?
    };

    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failures = Vec::new();

    for test_file in &test_files {
        total += 1;
        let output = Command::new("bash")
            .arg(test_file)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    passed += 1;
                } else {
                    failed += 1;
                    let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
                    let stdout_full = String::from_utf8_lossy(&result.stdout).trim().to_string();
                    let detail = if !stderr.is_empty() { stderr } else { stdout_full };
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

    let result = TestOutput { total, passed, failed, failures };

    if human {
        println!("[dev-flow] 测试结果：{}/{} 通过", passed, total);
        if !result.failures.is_empty() {
            println!("失败：");
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
