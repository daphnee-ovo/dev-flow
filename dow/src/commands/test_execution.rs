use crate::error::DowError;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

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

#[derive(Debug, Clone, Copy)]
pub(crate) enum TestStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub(crate) enum TestProgress {
    Started { label: String },
    Output { text: String },
    Finished {
        label: String,
        outcome: TestOutcome,
    },
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

pub(crate) fn execute_plans_with_progress(
    plans: &[TestPlan],
    mut on_progress: impl FnMut(TestProgress),
) -> TestSummary {
    let mut summary = TestSummary {
        total: plans.len(),
        ..TestSummary::default()
    };

    for plan in plans {
        on_progress(TestProgress::Started {
            label: plan.label.clone(),
        });

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
            on_progress(TestProgress::Output {
                text: format!("{}\n", reason),
            });
            on_progress(TestProgress::Finished {
                label: plan.label.clone(),
                outcome: TestOutcome::PreconditionFailed,
            });
            continue;
        }

        match execute_plan(plan, &mut on_progress) {
            Ok((status, _stdout, _stderr)) if status.success() => {
                summary.passed += 1;
                on_progress(TestProgress::Finished {
                    label: plan.label.clone(),
                    outcome: TestOutcome::Pass,
                });
            }
            Ok((status, stdout, stderr)) => {
                let stdout = String::from_utf8_lossy(&stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
                let command_not_found = status.code() == Some(127)
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
                on_progress(TestProgress::Finished {
                    label: plan.label.clone(),
                    outcome: if command_not_found {
                        TestOutcome::PreconditionFailed
                    } else {
                        TestOutcome::TestFailed
                    },
                });
            }
            Err(error) => {
                summary.precondition_failed += 1;
                on_progress(TestProgress::Output {
                    text: format!("{}\n", error),
                });
                summary.failures.push(TestFailure {
                    label: plan.label.clone(),
                    command: plan.command.clone(),
                    files: plan.files.clone(),
                    stdout: String::new(),
                    stderr: error.to_string(),
                    precondition: true,
                });
                on_progress(TestProgress::Finished {
                    label: plan.label.clone(),
                    outcome: TestOutcome::PreconditionFailed,
                });
            }
        }
    }

    summary
}

fn execute_plan(
    plan: &TestPlan,
    on_progress: &mut impl FnMut(TestProgress),
) -> std::io::Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>)> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&plan.command)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("piped child stdout");
    let stderr = child.stderr.take().expect("piped child stderr");
    let (sender, receiver) = mpsc::channel();
    let stdout_reader = spawn_output_reader(stdout, TestStream::Stdout, sender.clone());
    let stderr_reader = spawn_output_reader(stderr, TestStream::Stderr, sender.clone());
    drop(sender);

    let mut stdout_output = Vec::new();
    let mut stderr_output = Vec::new();
    for (stream, line) in receiver {
        let text = String::from_utf8_lossy(&line).into_owned();
        match stream {
            TestStream::Stdout => stdout_output.extend_from_slice(&line),
            TestStream::Stderr => stderr_output.extend_from_slice(&line),
        }
        on_progress(TestProgress::Output {
            text,
        });
    }

    let status = child.wait()?;
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    Ok((status, stdout_output, stderr_output))
}

fn spawn_output_reader<R>(
    reader: R,
    stream: TestStream,
    sender: mpsc::Sender<(TestStream, Vec<u8>)>,
) -> thread::JoinHandle<()>
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if sender.send((stream, line)).is_err() {
                        break;
                    }
                }
            }
        }
    })
}
