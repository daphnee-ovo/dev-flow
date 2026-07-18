use super::test_config::TestCiConfig;
use super::test_execution::{expand_command, shell_quote, TestPlan};
use crate::core::doc_root;
use crate::error::DowError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub(crate) enum TestTarget {
    Full,
    Task(String),
}

#[derive(Debug, Clone)]
pub(crate) struct TaskTestContext {
    pub(crate) id: String,
    pub(crate) task_file: PathBuf,
    pub(crate) test_files: Vec<String>,
}

pub(crate) fn build_plans(
    target: &TestTarget,
    project_root: &Path,
    config: Option<&TestCiConfig>,
) -> Result<Vec<TestPlan>, DowError> {
    let (custom_commands, task_context) = match target {
        TestTarget::Full => (
            config.map(|item| item.test.clone()).unwrap_or_default(),
            None,
        ),
        TestTarget::Task(id) => {
            let context = find_task_context(id, project_root)?;
            (
                config.map(|item| item.devtest.clone()).unwrap_or_default(),
                Some(context),
            )
        }
    };

    let task_id = task_context.as_ref().map(|context| context.id.as_str());
    let task_file = task_context
        .as_ref()
        .map(|context| context.task_file.to_string_lossy().to_string());
    let test_files = task_context
        .as_ref()
        .map(|context| context.test_files.clone())
        .unwrap_or_default();

    if !custom_commands.is_empty() {
        return custom_commands
            .iter()
            .enumerate()
            .map(|(index, template)| {
                Ok(TestPlan {
                    label: format!("test.ci run {}", index + 1),
                    command: expand_command(
                        template,
                        &project_root.to_string_lossy(),
                        task_id,
                        task_file.as_deref(),
                        &test_files,
                    )?,
                    cwd: project_root.to_path_buf(),
                    files: test_files.clone(),
                    precondition: None,
                })
            })
            .collect();
    }

    match target {
        TestTarget::Full => build_full_plans(project_root),
        TestTarget::Task(_) => build_task_plans(project_root, task_context.unwrap()),
    }
}

fn build_full_plans(project_root: &Path) -> Result<Vec<TestPlan>, DowError> {
    let mut plans = Vec::new();

    if project_root.join("Cargo.toml").exists() {
        let command = if fs::read_to_string(project_root.join("Cargo.toml"))
            .map(|content| content.contains("[workspace]"))
            .unwrap_or(false)
        {
            "cargo test --workspace"
        } else {
            "cargo test"
        };
        plans.push(command_plan(
            "Rust project",
            command,
            project_root,
            vec![],
            "cargo",
        ));
    }

    if project_root.join("go.mod").exists() {
        plans.push(command_plan(
            "Go module",
            "go test ./...",
            project_root,
            vec![],
            "go",
        ));
    }

    if has_python_test_config(project_root) {
        plans.push(python_plan(project_root, None, "Python pytest"));
    }

    if project_root.join("package.json").exists() {
        plans.push(javascript_full_plan(project_root));
    }

    let shell_tests = discover_shell_tests(project_root);
    for file in shell_tests {
        plans.push(command_plan(
            &format!("Shell {}", file),
            &format!("bash {}", shell_quote(&file)),
            project_root,
            vec![file],
            "bash",
        ));
    }

    Ok(plans)
}

fn build_task_plans(
    project_root: &Path,
    context: TaskTestContext,
) -> Result<Vec<TestPlan>, DowError> {
    let mut plans = Vec::new();
    for file in context.test_files {
        let path = project_root.join(&file);
        if !path.exists() {
            plans.push(TestPlan {
                label: file.clone(),
                command: String::new(),
                cwd: project_root.to_path_buf(),
                files: vec![file.clone()],
                precondition: Some(format!("test file does not exist: {}", file)),
            });
            continue;
        }

        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let plan = match extension {
            "rs" => rust_plan(project_root, &path, &file),
            "go" if file.ends_with("_test.go") => go_plan(project_root, &path, &file),
            "py" => python_plan(project_root, Some(&file), &format!("Python {}", file)),
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => {
                javascript_task_plan(project_root, &path, &file)
            }
            "sh" => command_plan(
                &format!("Shell {}", file),
                &format!("bash {}", shell_quote(&file)),
                project_root,
                vec![file.clone()],
                "bash",
            ),
            _ => TestPlan {
                label: file.clone(),
                command: String::new(),
                cwd: project_root.to_path_buf(),
                files: vec![file.clone()],
                precondition: Some(format!(
                    "No built-in test adapter for {}. Please configure devtest.run in .dev-doc/test.ci.",
                    file
                )),
            },
        };
        plans.push(plan);
    }
    Ok(plans)
}

fn rust_plan(project_root: &Path, path: &Path, display_file: &str) -> TestPlan {
    let Some(manifest) = find_upward(path, "Cargo.toml") else {
        return TestPlan {
            label: display_file.to_string(),
            command: String::new(),
            cwd: project_root.to_path_buf(),
            files: vec![display_file.to_string()],
            precondition: Some(format!("Cargo manifest not found for {}", display_file)),
        };
    };

    let crate_root = manifest.parent().unwrap_or(project_root);
    let is_integration = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        == Some("tests");
    let command = if is_integration {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        format!(
            "cargo test --manifest-path {} --test {}",
            shell_quote(&manifest.to_string_lossy()),
            shell_quote(stem)
        )
    } else {
        format!(
            "cargo test --manifest-path {}",
            shell_quote(&manifest.to_string_lossy())
        )
    };

    let label = if is_integration {
        format!("Rust integration {}", display_file)
    } else {
        format!("Rust inline crate fallback {}", display_file)
    };
    command_plan(
        &label,
        &command,
        crate_root,
        vec![display_file.to_string()],
        "cargo",
    )
}

fn go_plan(project_root: &Path, path: &Path, display_file: &str) -> TestPlan {
    let Some(module_file) = find_upward(path, "go.mod") else {
        return TestPlan {
            label: display_file.to_string(),
            command: String::new(),
            cwd: project_root.to_path_buf(),
            files: vec![display_file.to_string()],
            precondition: Some(format!("go.mod not found for {}", display_file)),
        };
    };
    let module_root = module_file.parent().unwrap_or(project_root);
    let package_dir = path.parent().unwrap_or(module_root);
    let package = package_dir
        .strip_prefix(module_root)
        .ok()
        .map(|relative| {
            relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/")
        })
        .filter(|relative| !relative.is_empty())
        .unwrap_or_else(|| ".".to_string());

    command_plan(
        &format!("Go package {}", display_file),
        &format!("go test ./{}", package.trim_start_matches("./")),
        module_root,
        vec![display_file.to_string()],
        "go",
    )
}

fn python_plan(project_root: &Path, file: Option<&String>, label: &str) -> TestPlan {
    let python = resolve_python(project_root);
    let command = match file {
        Some(file) => format!("{} -m pytest {}", shell_quote(&python), shell_quote(file)),
        None => format!("{} -m pytest", shell_quote(&python)),
    };
    let files = file.into_iter().cloned().collect::<Vec<_>>();
    let mut plan = command_plan(label, &command, project_root, files, &python);
    if plan.precondition.is_none() && !python_pytest_available(&python) {
        plan.precondition = Some(format!("pytest is not available via {}", python));
    }
    plan
}

fn python_pytest_available(python: &str) -> bool {
    Command::new(python)
        .args(["-c", "import pytest"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Resolve the best available Python interpreter for a project.
/// Priority: .venv/bin/python (project-local) > python3 (system) > python (fallback).
fn resolve_python(project_root: &Path) -> String {
    let venv_python = project_root.join(".venv/bin/python");
    if venv_python.exists() {
        return venv_python.to_string_lossy().to_string();
    }
    if command_exists("python3") {
        return "python3".to_string();
    }
    "python".to_string()
}

fn javascript_full_plan(project_root: &Path) -> TestPlan {
    let package = match read_package_json(project_root) {
        Ok(package) => package,
        Err(error) => return precondition_plan("JavaScript/TypeScript", error),
    };
    let has_test_script = package
        .get("scripts")
        .and_then(|scripts| scripts.get("test"))
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !has_test_script {
        return precondition_plan(
            "JavaScript/TypeScript",
            "package.json does not define scripts.test; configure test.ci.test.run",
        );
    }

    let manager = match package_manager(project_root, &package) {
        Ok(manager) => manager,
        Err(error) => return precondition_plan("JavaScript/TypeScript", error),
    };
    let command = match manager.as_str() {
        "npm" => "npm test",
        "pnpm" => "pnpm test",
        "yarn" => "yarn test",
        "bun" => "bun run test",
        _ => unreachable!(),
    };
    command_plan(
        "JavaScript/TypeScript package test",
        command,
        project_root,
        vec![],
        &manager,
    )
}

fn javascript_task_plan(project_root: &Path, path: &Path, display_file: &str) -> TestPlan {
    let Some(package_file) = find_upward(path, "package.json") else {
        return precondition_plan(
            display_file,
            format!("package.json not found for {}", display_file),
        );
    };
    let package_root = package_file.parent().unwrap_or(project_root);
    let package = match read_package_json(package_root) {
        Ok(package) => package,
        Err(error) => return precondition_plan(display_file, error),
    };
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let runner = if dependency_exists(&package, "vitest") {
        Some("vitest")
    } else if dependency_exists(&package, "jest") {
        Some("jest")
    } else if matches!(extension, "js" | "jsx" | "mjs" | "cjs") {
        Some("node")
    } else {
        None
    };
    let Some(runner) = runner else {
        return precondition_plan(
            display_file,
            format!(
                "No built-in JavaScript/TypeScript runner for {}. Please configure devtest.run in .dev-doc/test.ci.",
                display_file
            ),
        );
    };

    let command = if runner == "node" {
        format!("node --test {}", shell_quote(display_file))
    } else {
        let manager = match package_manager(package_root, &package) {
            Ok(manager) => manager,
            Err(error) => return precondition_plan(display_file, error),
        };
        package_exec_command(&manager, runner, display_file)
    };

    let required_command = if runner == "node" {
        "node".to_string()
    } else {
        match package_manager(package_root, &package) {
            Ok(manager) => manager,
            Err(error) => return precondition_plan(display_file, error),
        }
    };
    command_plan(
        &format!("{} {}", runner, display_file),
        &command,
        package_root,
        vec![display_file.to_string()],
        &required_command,
    )
}

fn package_exec_command(manager: &str, runner: &str, file: &str) -> String {
    match manager {
        "npm" => format!("npm exec -- {} run {}", runner, shell_quote(file)),
        "pnpm" => format!("pnpm exec {} run {}", runner, shell_quote(file)),
        "yarn" => format!("yarn exec {} run {}", runner, shell_quote(file)),
        "bun" => format!("bunx --no-install {} run {}", runner, shell_quote(file)),
        _ => unreachable!(),
    }
}

fn read_package_json(project_root: &Path) -> Result<serde_json::Value, String> {
    let path = project_root.join("package.json");
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| format!("invalid package.json: {}", error))
}

fn dependency_exists(package: &serde_json::Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|field| {
            package
                .get(field)
                .and_then(|value| value.get(name))
                .is_some()
        })
}

fn package_manager(project_root: &Path, package: &serde_json::Value) -> Result<String, String> {
    if let Some(value) = package
        .get("packageManager")
        .and_then(|value| value.as_str())
    {
        if let Some(manager) = value.split('@').next() {
            if ["npm", "pnpm", "yarn", "bun"].contains(&manager) {
                return Ok(manager.to_string());
            }
        }
        return Err(format!("unsupported packageManager: {}", value));
    }

    let lockfiles = [
        ("package-lock.json", "npm"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
    ];
    let managers: Vec<&str> = lockfiles
        .iter()
        .filter(|(file, _)| project_root.join(file).exists())
        .map(|(_, manager)| *manager)
        .collect();
    match managers.as_slice() {
        [manager] => Ok((*manager).to_string()),
        [] => Err("cannot determine package manager from package.json or lockfile".to_string()),
        _ => Err("multiple package manager lockfiles found".to_string()),
    }
}

fn precondition_plan(label: &str, reason: impl Into<String>) -> TestPlan {
    TestPlan {
        label: label.to_string(),
        command: String::new(),
        cwd: PathBuf::from("."),
        files: vec![],
        precondition: Some(reason.into()),
    }
}

fn command_plan(
    label: &str,
    command: &str,
    cwd: &Path,
    files: Vec<String>,
    required_command: &str,
) -> TestPlan {
    TestPlan {
        label: label.to_string(),
        command: command.to_string(),
        cwd: cwd.to_path_buf(),
        files,
        precondition: if command_exists(required_command) {
            None
        } else {
            Some(format!(
                "required test tool is not available: {}",
                required_command
            ))
        },
    }
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", command)])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn discover_shell_tests(project_root: &Path) -> Vec<String> {
    let mut files = fs::read_dir(project_root.join("tests"))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("test_") && name.ends_with(".sh") && name != "test_all.sh" {
                Some(format!("tests/{}", name))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn has_python_test_config(project_root: &Path) -> bool {
    if project_root.join("pytest.ini").exists() {
        return true;
    }
    if project_root.join("pyproject.toml").exists()
        && fs::read_to_string(project_root.join("pyproject.toml"))
            .map(|content| content.contains("[tool.pytest"))
            .unwrap_or(false)
    {
        return true;
    }
    project_root.join("setup.cfg").exists()
        && fs::read_to_string(project_root.join("setup.cfg"))
            .map(|content| content.contains("[tool:pytest]"))
            .unwrap_or(false)
}

fn find_upward(path: &Path, filename: &str) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        let candidate = current.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn extract_test_files(lines: &[&str], task_line: usize, _project_root: &Path) -> Vec<String> {
    let mut in_test = false;
    let mut files = Vec::new();

    for line in lines.iter().skip(task_line + 1) {
        if line.starts_with("- [") {
            break;
        }

        let trimmed = line.trim();
        if trimmed.starts_with("test:") {
            in_test = true;
            if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed[start + 1..].find(']') {
                    let inner = &trimmed[start + 1..start + 1 + end];
                    files.extend(
                        inner
                            .split(',')
                            .map(|item| item.trim().trim_matches('"').trim_matches('\''))
                            .filter(|item| !item.is_empty())
                            .map(ToOwned::to_owned),
                    );
                    in_test = false;
                }
            }
            continue;
        }

        if in_test {
            if trimmed.starts_with('-') {
                let value = trimmed.trim_start_matches('-').trim();
                if !value.is_empty() {
                    files.push(value.trim_matches('"').trim_matches('\'').to_string());
                }
            } else if !trimmed.is_empty() {
                in_test = false;
            }
        }
    }

    files
}

fn find_task_context(task_id: &str, _project_root: &Path) -> Result<TaskTestContext, DowError> {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let task_dir = doc_root_path.join("task");
    if !task_dir.is_dir() {
        return Err(DowError::new("task/ directory does not exist", 2));
    }

    let normalized = if task_id.starts_with("TASK-") {
        task_id.to_string()
    } else {
        format!("TASK-{}", task_id)
    };

    let mut entries: Vec<_> = fs::read_dir(&task_dir)
        .map_err(|e| DowError::new(format!("cannot read task directory: {}", e), 2))?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.ends_with(".md") && (name.starts_with("task_") || name.starts_with("done_task_"))
        })
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries.iter().rev() {
        let content = fs::read_to_string(entry.path())
            .map_err(|e| DowError::new(format!("cannot read task file: {}", e), 2))?;
        let lines: Vec<&str> = content.lines().collect();
        for (line_number, line) in lines.iter().enumerate() {
            if !(line.starts_with("- [ ]") || line.starts_with("- [x]"))
                || !line.contains(&normalized)
            {
                continue;
            }

            return Ok(TaskTestContext {
                id: normalized,
                task_file: entry.path(),
                test_files: extract_test_files(&lines, line_number, _project_root),
            });
        }
    }

    Err(DowError::new(
        format!("Task {} not found in active or done task files", task_id),
        2,
    ))
}
