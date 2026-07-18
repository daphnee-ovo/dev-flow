// dow/src/commands/
// ├── info.rs  -- dow info context (Generate project context summary for agent subagents)

use crate::error::DowError;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn context() -> Result<i32, DowError> {
    let root = std::env::current_dir().map_err(|e| DowError::new(e.to_string(), 1))?;
    let output = generate_context(&root);
    print!("{}", output);
    Ok(0)
}

fn generate_context(root: &Path) -> String {
    let mut out = String::from("# Project Context\n");

    if !root.is_dir() {
        out.push_str("(Directory does not exist)\n");
        return out;
    }

    // Empty project detection
    let has_files = fs::read_dir(root)
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name != ".git"
            })
        })
        .unwrap_or(false);

    if !has_files {
        out.push_str("(Empty project)\n");
        return out;
    }

    // Tech stack
    out.push_str("\n## Tech Stack\n");
    let mut stack = Vec::new();
    if root.join("package.json").exists() {
        stack.push("Node.js/JavaScript");
    }
    if root.join("tsconfig.json").exists() {
        stack.push("TypeScript");
    }
    if root.join("requirements.txt").exists()
        || root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
    {
        stack.push("Python");
    }
    if root.join("Cargo.toml").exists() {
        stack.push("Rust");
    }
    if root.join("go.mod").exists() {
        stack.push("Go");
    }
    if root.join("Gemfile").exists() {
        stack.push("Ruby");
    }
    if root.join("pom.xml").exists() || root.join("build.gradle").exists() {
        stack.push("Java");
    }
    if root.join("Dockerfile").exists() {
        stack.push("Docker");
    }
    if stack.is_empty() {
        out.push_str("- (Unable to auto-detect)\n");
    } else {
        for s in &stack {
            out.push_str(&format!("- {}\n", s));
        }
    }

    // Directory structure
    out.push_str("\n## Directory Structure\n");
    if let Ok(result) = Command::new("tree")
        .args([
            root.to_str().unwrap_or("."),
            "-L",
            "2",
            "--dirsfirst",
            "-I",
            "node_modules|.git|__pycache__|.venv|venv|dist|build|.codegraph|tmp|temp|target",
            "--noreport",
        ])
        .output()
    {
        let tree = String::from_utf8_lossy(&result.stdout);
        let lines: Vec<&str> = tree.lines().take(60).collect();
        out.push_str(&lines.join("\n"));
        out.push('\n');
    } else {
        out.push_str("(tree command not available)\n");
    }

    // Existing tests
    let test_dir = if root.join("tests").is_dir() {
        Some(root.join("tests"))
    } else if root.join("test").is_dir() {
        Some(root.join("test"))
    } else {
        None
    };
    if let Some(ref td) = test_dir {
        out.push_str("\n## Existing Tests\n");
        let mut test_files = Vec::new();
        collect_test_files(td, root, &mut test_files, 20);
        if test_files.is_empty() {
            out.push_str("(tests/ directory exists but no matching test files found)\n");
        } else {
            for f in &test_files {
                out.push_str(&format!("- {}\n", f));
            }
        }
    }

    // How to run
    out.push_str("\n## How to Run\n");
    let mut run_info = Vec::new();
    if root.join("Makefile").exists() {
        run_info.push("Makefile available (make)");
    }
    if root.join("package.json").exists() {
        run_info.push("npm scripts available (npm run)");
    }
    if root.join("Dockerfile").exists() {
        run_info.push("Docker build available");
    }
    if root.join("Cargo.toml").exists() {
        run_info.push("cargo build/run available");
    }
    if run_info.is_empty() {
        out.push_str("- (No standard run entry)\n");
    } else {
        for r in &run_info {
            out.push_str(&format!("- {}\n", r));
        }
    }

    // Core modules
    out.push_str("\n## Core Modules\n");
    let mut modules = Vec::new();
    for dir_name in &["src", "lib", "scripts", "commands", "agents"] {
        let dir = root.join(dir_name);
        if dir.is_dir() {
            let count = count_files(&dir);
            modules.push(format!("- {}/ ({} files)", dir_name, count));
        }
    }
    if modules.is_empty() {
        out.push_str("- (No standard module directories)\n");
    } else {
        for m in &modules {
            out.push_str(m);
            out.push('\n');
        }
    }

    // Truncate to 200 lines
    let lines: Vec<&str> = out.lines().take(200).collect();
    lines.join("\n") + "\n"
}

fn collect_test_files(dir: &Path, root: &Path, results: &mut Vec<String>, max: usize) {
    if results.len() >= max {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());
        for entry in sorted {
            if results.len() >= max {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                collect_test_files(&path, root, results, max);
            } else {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("test_") || name.contains("_test.") || name.contains(".test.") {
                    let rel = path
                        .strip_prefix(root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or(name);
                    results.push(rel);
                }
            }
        }
    }
}

fn count_files(dir: &Path) -> u32 {
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                count += count_files(&path);
            }
        }
    }
    count
}
