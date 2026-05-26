// dow/src/commands/
// ├── scan.rs  -- dow scan（项目扫描，替代 scan-project.sh）

use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct ScanOutput {
    name: String,
    tech_stack: Vec<String>,
    commands: ScanCommands,
    style: Vec<String>,
    structure: Vec<String>,
    file_count: usize,
    readme_first_line: Option<String>,
    git: GitInfo,
    dev_doc: DevDocInfo,
    agent_files: Vec<String>,
}

#[derive(Serialize)]
struct ScanCommands {
    #[serde(skip_serializing_if = "Option::is_none")]
    package_json_scripts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    makefile_targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pytest: Option<bool>,
}

#[derive(Serialize)]
struct GitInfo {
    branch: String,
    commits: u32,
    recent: Vec<String>,
}

#[derive(Serialize)]
struct DevDocInfo {
    exists: bool,
    files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_summary: Option<String>,
}

pub fn run(human: bool) -> Result<i32, DowError> {
    let scan = ScanOutput {
        name: detect_name(),
        tech_stack: detect_stack(),
        commands: detect_commands(),
        style: detect_style(),
        structure: detect_structure(),
        file_count: count_files(),
        readme_first_line: read_readme_first(),
        git: detect_git(),
        dev_doc: detect_dev_doc(),
        agent_files: detect_agent_files(),
    };

    if human {
        print_human(&scan);
    } else {
        output::print_json(&scan);
    }

    Ok(0)
}

fn detect_name() -> String {
    // package.json
    if let Ok(content) = fs::read_to_string("package.json") {
        if let Some(name) = extract_json_field(&content, "name") {
            return name;
        }
    }
    // pyproject.toml
    if let Ok(content) = fs::read_to_string("pyproject.toml") {
        for line in content.lines() {
            if line.starts_with("name") && line.contains('=') {
                if let Some(val) = line.split('"').nth(1) {
                    return val.to_string();
                }
            }
        }
    }
    // Cargo.toml
    if let Ok(content) = fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            if line.starts_with("name") && line.contains('=') {
                if let Some(val) = line.split('"').nth(1) {
                    return val.to_string();
                }
            }
        }
    }
    // fallback: 目录名
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn detect_stack() -> Vec<String> {
    let mut stack = Vec::new();
    if Path::new("package.json").exists() {
        stack.push("node".to_string());
    }
    if Path::new("tsconfig.json").exists() {
        stack.push("typescript".to_string());
    }
    if Path::new("next.config.js").exists()
        || Path::new("next.config.ts").exists()
        || Path::new("next.config.mjs").exists()
    {
        stack.push("nextjs".to_string());
    }
    if Path::new("pyproject.toml").exists()
        || Path::new("setup.py").exists()
        || Path::new("requirements.txt").exists()
    {
        stack.push("python".to_string());
    }
    if Path::new("go.mod").exists() {
        stack.push("go".to_string());
    }
    if Path::new("Cargo.toml").exists() || Path::new("dow/Cargo.toml").exists() {
        stack.push("rust".to_string());
    }
    if Path::new("Gemfile").exists() {
        stack.push("ruby".to_string());
    }
    if Path::new("pom.xml").exists() || Path::new("build.gradle").exists() {
        stack.push("java".to_string());
    }
    stack
}

fn detect_commands() -> ScanCommands {
    let package_json_scripts = if Path::new("package.json").exists() {
        if let Ok(content) = fs::read_to_string("package.json") {
            let scripts: Vec<String> = content
                .lines()
                .filter(|l| {
                    let trimmed = l.trim();
                    trimmed.contains("\"build\"")
                        || trimmed.contains("\"test\"")
                        || trimmed.contains("\"dev\"")
                        || trimmed.contains("\"start\"")
                        || trimmed.contains("\"lint\"")
                })
                .map(|l| l.trim().trim_end_matches(',').to_string())
                .collect();
            if scripts.is_empty() { None } else { Some(scripts) }
        } else {
            None
        }
    } else {
        None
    };

    let makefile_targets = if Path::new("Makefile").exists() {
        if let Ok(content) = fs::read_to_string("Makefile") {
            let targets: Vec<String> = content
                .lines()
                .filter(|l| l.contains(':') && !l.starts_with('\t') && !l.starts_with('#'))
                .take(10)
                .filter_map(|l| l.split(':').next().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty() && !s.contains(' '))
                .collect();
            if targets.is_empty() { None } else { Some(targets) }
        } else {
            None
        }
    } else {
        None
    };

    let pytest = if Path::new("pyproject.toml").exists() {
        fs::read_to_string("pyproject.toml")
            .ok()
            .map(|c| c.contains("[tool.pytest"))
    } else {
        None
    };

    ScanCommands {
        package_json_scripts,
        makefile_targets,
        pytest,
    }
}

fn detect_style() -> Vec<String> {
    let mut style = Vec::new();
    if Path::new(".eslintrc").exists()
        || Path::new(".eslintrc.js").exists()
        || Path::new(".eslintrc.json").exists()
        || Path::new("eslint.config.js").exists()
    {
        style.push("eslint".to_string());
    }
    if Path::new(".prettierrc").exists()
        || Path::new(".prettierrc.js").exists()
        || Path::new("prettier.config.js").exists()
    {
        style.push("prettier".to_string());
    }
    if Path::new("ruff.toml").exists() {
        style.push("ruff".to_string());
    }
    if Path::new(".editorconfig").exists() {
        style.push("editorconfig".to_string());
    }
    if Path::new("biome.json").exists() {
        style.push("biome".to_string());
    }
    style
}

fn detect_structure() -> Vec<String> {
    let mut dirs = Vec::new();
    if let Ok(entries) = fs::read_dir(".") {
        let mut entries: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(
                    name.as_str(),
                    ".git" | "node_modules" | ".next" | "__pycache__" | "tmp" | "temp" | "target"
                )
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        entries.sort();
        dirs = entries;
    }
    dirs
}

fn count_files() -> usize {
    let output = Command::new("find")
        .args([".", "-type", "f",
            "!", "-path", "./.git/*",
            "!", "-path", "./node_modules/*",
            "!", "-path", "./tmp/*",
            "!", "-path", "./temp/*",
            "!", "-path", "./.next/*",
            "!", "-path", "./target/*",
            "!", "-path", "./dow/target/*",
        ])
        .output();

    output
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0)
}

fn read_readme_first() -> Option<String> {
    fs::read_to_string("README.md")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
        })
}

fn detect_git() -> GitInfo {
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let commits = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u32>()
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let recent = Command::new("git")
        .args(["log", "--oneline", "-5"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default();

    GitInfo { branch, commits, recent }
}

fn detect_dev_doc() -> DevDocInfo {
    if !Path::new("dev-doc").is_dir() {
        return DevDocInfo {
            exists: false,
            files: Vec::new(),
            task_summary: None,
            issue_summary: None,
        };
    }

    let files: Vec<String> = walkdir_simple("dev-doc")
        .into_iter()
        .filter(|f| f.ends_with(".md") || f.ends_with(".yaml"))
        .collect();

    let task_summary = if Path::new("dev-doc/task").is_dir() {
        let active = fs::read_dir("dev-doc/task")
            .map(|e| {
                e.flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.starts_with("task_") && n.ends_with(".md")
                    })
                    .count()
            })
            .unwrap_or(0);
        let done = fs::read_dir("dev-doc/task")
            .map(|e| {
                e.flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.starts_with("done_task_") && n.ends_with(".md")
                    })
                    .count()
            })
            .unwrap_or(0);
        Some(format!("active={} done={}", active, done))
    } else {
        None
    };

    let issue_summary = if Path::new("dev-doc/issue").is_dir() {
        let open = fs::read_dir("dev-doc/issue")
            .map(|e| {
                e.flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.starts_with("issue_") && n.ends_with(".md")
                    })
                    .count()
            })
            .unwrap_or(0);
        let closed = fs::read_dir("dev-doc/issue")
            .map(|e| {
                e.flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.starts_with("closed_issue_") && n.ends_with(".md")
                    })
                    .count()
            })
            .unwrap_or(0);
        Some(format!("open={} closed={}", open, closed))
    } else {
        None
    };

    DevDocInfo {
        exists: true,
        files,
        task_summary,
        issue_summary,
    }
}

fn detect_agent_files() -> Vec<String> {
    let mut files = Vec::new();
    for f in &["CLAUDE.md", "AGENTS.md", ".cursorrules", ".windsurfrules"] {
        if Path::new(f).exists() {
            files.push(f.to_string());
        }
    }
    files
}

fn walkdir_simple(dir: &str) -> Vec<String> {
    let mut result = Vec::new();
    fn walk(path: &Path, result: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, result);
                } else {
                    result.push(p.to_string_lossy().to_string());
                }
            }
        }
    }
    walk(Path::new(dir), &mut result);
    result.sort();
    result
}

fn extract_json_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    for line in content.lines() {
        if line.contains(&pattern) {
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 4 {
                return Some(parts[3].to_string());
            }
        }
    }
    None
}

fn print_human(scan: &ScanOutput) {
    println!("=== PROJECT SCAN ===");
    println!("name: {}", scan.name);
    println!("stack: {}", scan.tech_stack.join(" "));
    println!("structure: {}", scan.structure.join(", "));
    println!("file_count: {}", scan.file_count);
    if let Some(ref readme) = scan.readme_first_line {
        println!("readme: {}", readme);
    }
    println!("git: branch={} commits={}", scan.git.branch, scan.git.commits);
    println!("dev_doc: exists={}", scan.dev_doc.exists);
    println!("agent_files: {}", scan.agent_files.join(", "));
}
