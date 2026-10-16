use super::test_adapters::{self, TestTarget};
use super::test_config;
use super::test_execution::{
    execute_plans_with_progress, TestFailure, TestOutcome, TestProgress, TestSummary,
};
use crate::cli::TestArgs;
use crate::commands::issue::{create_issue_batch, IssueCreateRecord};
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestRunStatus {
    Pass,
    TestFailed,
    PreconditionFailed,
}

impl From<TestOutcome> for TestRunStatus {
    fn from(value: TestOutcome) -> Self {
        match value {
            TestOutcome::Pass => Self::Pass,
            TestOutcome::TestFailed => Self::TestFailed,
            TestOutcome::PreconditionFailed => Self::PreconditionFailed,
        }
    }
}

#[derive(Serialize)]
struct TestOutput<'a> {
    target: &'a str,
    outcome: &'static str,
    total: usize,
    passed: usize,
    test_failed: usize,
    precondition_failed: usize,
    failures: &'a [TestFailure],
}

struct RunResult {
    status: TestRunStatus,
    summary: TestSummary,
}

pub(crate) struct TaskTestCheck {
    pub(crate) has_tests: bool,
    pub(crate) passed: bool,
    pub(crate) exit_code: i32,
}

pub fn run(args: TestArgs, human: bool) -> Result<i32, DowError> {
    let target = args
        .task_id
        .map(TestTarget::Task)
        .unwrap_or(TestTarget::Full);
    let result = execute_target(&target);
    print_result(&target, &result, human);
    Ok(result.summary.exit_code())
}

pub(crate) fn check_task_test(task_id: &str) -> Result<(), DowError> {
    let target = TestTarget::Task(task_id.to_string());
    let result = execute_target(&target);
    if result.status == TestRunStatus::Pass {
        return Ok(());
    }

    Err(DowError::new(
        failure_message(&result.summary),
        result.summary.exit_code(),
    ))
}

pub(crate) fn check_task_test_verbose(
    task_id: &str,
    on_progress: impl FnMut(TestProgress),
) -> TaskTestCheck {
    let target = TestTarget::Task(task_id.to_string());
    let result = execute_target_with_progress(&target, on_progress);
    TaskTestCheck {
        has_tests: result.summary.total > 0,
        passed: result.status == TestRunStatus::Pass,
        exit_code: result.summary.exit_code(),
    }
}

fn execute_target(target: &TestTarget) -> RunResult {
    execute_target_with_progress(target, |_| {})
}

fn execute_target_with_progress(
    target: &TestTarget,
    on_progress: impl FnMut(TestProgress),
) -> RunResult {
    let project_root = doc_root::project_root();
    let config = match test_config::load(&project_root) {
        Ok(config) => config,
        Err(error) => {
            return precondition_result("test.ci", error.to_string(), on_progress);
        }
    };

    let plans = match test_adapters::build_plans(target, &project_root, config.as_ref()) {
        Ok(plans) => plans,
        Err(error) => {
            return precondition_result("test target", error.to_string(), on_progress);
        }
    };

    let summary = execute_plans_with_progress(&plans, on_progress);
    let status = TestRunStatus::from(summary.outcome());
    if status == TestRunStatus::TestFailed {
        create_test_issues(target, &summary, &project_root);
    }

    RunResult { status, summary }
}

fn failure_message(summary: &TestSummary) -> String {
    summary
        .failures
        .iter()
        .map(TestFailure::raw_message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn precondition_result(
    label: &str,
    message: String,
    mut on_progress: impl FnMut(TestProgress),
) -> RunResult {
    on_progress(TestProgress::Started {
        label: label.to_string(),
    });
    on_progress(TestProgress::Output {
        text: format!("{}\n", message),
    });
    on_progress(TestProgress::Finished {
        label: label.to_string(),
        outcome: TestOutcome::PreconditionFailed,
    });

    let failure = TestFailure {
        label: label.to_string(),
        command: String::new(),
        files: vec![],
        stdout: String::new(),
        stderr: message,
        precondition: true,
    };
    RunResult {
        status: TestRunStatus::PreconditionFailed,
        summary: TestSummary {
            total: 1,
            passed: 0,
            test_failed: 0,
            precondition_failed: 1,
            failures: vec![failure],
        },
    }
}

fn create_test_issues(target: &TestTarget, summary: &TestSummary, project_root: &std::path::Path) {
    let task_id = match target {
        TestTarget::Full => None,
        TestTarget::Task(id) => Some(id.as_str()),
    };
    let records: Vec<IssueCreateRecord> = summary
        .failures
        .iter()
        .filter(|failure| !failure.precondition)
        .map(|failure| {
            let summary_text = failure_summary(failure);
            let title = match task_id {
                Some(id) => format!("Test {} fail:{}", id, summary_text),
                None => format!("Test fail:{}", summary_text),
            };
            let location = failure
                .files
                .first()
                .cloned()
                .unwrap_or_else(|| failure.label.clone());
            let reproduce = if failure.command.is_empty() {
                format!("project_root: {}", project_root.display())
            } else {
                format!(
                    "{}\nproject_root: {}",
                    failure.command,
                    project_root.display()
                )
            };
            IssueCreateRecord {
                title,
                severity: "P1".to_string(),
                location,
                desc: failure.raw_message(),
                reproduce,
                source: "test".to_string(),
                files_modify: failure.files.clone(),
                files_create: vec![],
            }
        })
        .collect();

    if records.is_empty() {
        return;
    }
    if let Err(error) = create_issue_batch(records) {
        eprintln!("[dev-flow] failed to create test ISSUE: {}", error);
    }
}

fn failure_summary(failure: &TestFailure) -> String {
    failure
        .raw_message()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("test failure")
        .chars()
        .take(120)
        .collect()
}

fn print_result(target: &TestTarget, result: &RunResult, human: bool) {
    let target_label = match target {
        TestTarget::Full => "full",
        TestTarget::Task(id) => id.as_str(),
    };
    let outcome = match result.status {
        TestRunStatus::Pass => "PASS",
        TestRunStatus::TestFailed => "TEST_FAILED",
        TestRunStatus::PreconditionFailed => "PRECONDITION_FAILED",
    };

    if human {
        println!(
            "[dev-flow] {} {}: {}/{} passed",
            target_label, outcome, result.summary.passed, result.summary.total
        );
        for failure in &result.summary.failures {
            println!("[dev-flow] {}: {}", failure.label, failure.raw_message());
        }
    } else {
        output::print_json(&TestOutput {
            target: target_label,
            outcome,
            total: result.summary.total,
            passed: result.summary.passed,
            test_failed: result.summary.test_failed,
            precondition_failed: result.summary.precondition_failed,
            failures: &result.summary.failures,
        });
    }
}
