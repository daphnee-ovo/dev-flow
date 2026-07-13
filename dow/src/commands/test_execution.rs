use crate::error::DowError;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub(crate) struct TestPlan {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) cwd: PathBuf,
    pub(crate) files: Vec<String>,
    pub(crate) precondition: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum TestOutcome {
    Pass,
    TestFailed,
    PreconditionFailed,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TestFailure {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) files: Vec<String>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) precondition: bool,
}

impl TestFailure {
    pub(crate) fn raw_message(&self) -> String {
        let mut output = String::new();
        if !self.stdout.is_empty() {
            output.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.stderr);
        }
        if output.is_empty() {
            output.push_str("test command failed without output");
        }
        output
    }
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct TestSummary {
    pub(crate) total: usize,
    pub(crate) passed: usize,
    pub(crate) test_failed: usize,
    pub(crate) precondition_failed: usize,
    pub(crate) failures: Vec<TestFailure>,
}

impl TestSummary {
    pub(crate) fn outcome(&self) -> TestOutcome {
        if self.test_failed > 0 {
            TestOutcome::TestFailed
        } else if self.precondition_failed > 0 {
            TestOutcome::PreconditionFailed
        } else {
            TestOutcome::Pass
        }
    }

    pub(crate) fn exit_code(&self) -> i32 {
        match self.outcome() {
            TestOutcome::Pass => 0,
            TestOutcome::TestFailed => 1,
            TestOutcome::PreconditionFailed => 2,
        }
    }
}

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn expand_command(
    template: &str,
    project_root: &str,
    task_id: Option<&str>,
    task_file: Option<&str>,
    test_files: &[String],
) -> Result<String, DowError> {
    let mut command = template.to_string();
    let replacements = [
        ("{{project_root}}", Some(shell_quote(project_root))),
        ("{{task_id}}", task_id.map(shell_quote)),
        ("{{task_file}}", task_file.map(shell_quote)),
        (
            "{{test_files}}",
            Some(
                test_files
                    .iter()
                    .map(|file| shell_quote(file))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        ),
    ];

    for (placeholder, value) in replacements {
        if command.contains(placeholder) {
            let value = value.ok_or_else(|| {
                DowError::new(format!("{} requires a Task test target", placeholder), 2)
            })?;
            command = command.replace(placeholder, &value);
        }
    }

    if let Some(start) = command.find("{{") {
        if let Some(end) = command[start..].find("}}") {
            return Err(DowError::new(
                format!(
                    "unknown test.ci placeholder: {}",
                    &command[start..start + end + 2]
                ),
                2,
            ));
        }
    }

    Ok(command)
}

pub(crate) fn execute_plans(plans: &[TestPlan]) -> TestSummary {
    let mut summary = TestSummary {
        total: plans.len(),
        ..TestSummary::default()
    };

    for plan in plans {
        if let Some(reason) = &plan.precondition {
            summary.precondition_failed += 1;
            summary.failures.push(TestFailure {
                label: plan.label.clone(),
                command: plan.command.clone(),
                files: plan.files.clone(),
                stdout: String::new(),
                stderr: reason.clone(),
                precondition: true,
            });
            continue;
        }

        match Command::new("sh")
            .arg("-c")
            .arg(&plan.command)
            .current_dir(&plan.cwd)
            .output()
        {
            Ok(output) if output.status.success() => {
                summary.passed += 1;
            }
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let command_not_found = output.status.code() == Some(127)
                    && (stderr.contains("command not found") || stderr.contains("not found"));
                if command_not_found {
                    summary.precondition_failed += 1;
                } else {
                    summary.test_failed += 1;
                }
                summary.failures.push(TestFailure {
                    label: plan.label.clone(),
                    command: plan.command.clone(),
                    files: plan.files.clone(),
                    stdout,
                    stderr,
                    precondition: command_not_found,
                });
            }
            Err(error) => {
                summary.precondition_failed += 1;
                summary.failures.push(TestFailure {
                    label: plan.label.clone(),
                    command: plan.command.clone(),
                    files: plan.files.clone(),
                    stdout: String::new(),
                    stderr: error.to_string(),
                    precondition: true,
                });
            }
        }
    }

    summary
}
