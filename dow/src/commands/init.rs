// dow/src/commands/
// ├── init.rs  -- dow init (initialize dev-flow workflow management)

use crate::cli::InitArgs;
use crate::core::{doc_root, version, yaml};
use crate::error::DowError;
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
struct InitOutput {
    name: String,
    mode: String,
    phase: String,
    doc_root: String,
    version: String,
}

pub fn run(args: InitArgs, _human: bool) -> Result<i32, DowError> {
    let valid_modes = ["full", "quick", "fast", "mvp"];
    if !valid_modes.contains(&args.mode.as_str()) {
        return Err(DowError::new(
            format!("Invalid mode: {} (options: full/quick/fast/mvp)", args.mode),
            1,
        ));
    }

    let base_dir = std::path::Path::new(crate::core::DOC_DIR);
    fs::create_dir_all(base_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Parse multi-branch mode path: .dev-doc/<branch>/
    let branch = crate::core::doc_root::current_branch().unwrap_or_else(|| "main".to_string());
    let doc_root_path = base_dir.join(&branch);
    fs::create_dir_all(&doc_root_path).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Create directory structure (archive migrated to SQLite, no longer creating directory)
    for dir in &["issue", "task"] {
        fs::create_dir_all(doc_root_path.join(dir)).map_err(|e| DowError::new(e.to_string(), 1))?;
    }
    fs::create_dir_all("tests").map_err(|e| DowError::new(e.to_string(), 1))?;

    // tmp directory: if temp already exists, don't create tmp
    if !std::path::Path::new("temp").is_dir() {
        fs::create_dir_all("tmp").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Determine starting phase
    let phase = match args.mode.as_str() {
        "full" => "PRD",
        "quick" | "mvp" => "SPEC",
        "fast" => "TASK",
        _ => "DEV",
    };

    // Write STATUS.yaml
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let status_content = format!(
        "name: {}\nphase: {}\nmode: {}\nexec_mode: step\nupdated: \"{}\"\nstarted: \"{}\"\n",
        args.name, phase, args.mode, now, now
    );
    let status_path = doc_root_path.join("STATUS.yaml");
    if status_path.exists() {
        return Err(DowError::new(
            "STATUS.yaml already exists, please delete it first if you need to reinitialize",
            1,
        ));
    }
    fs::write(&status_path, &status_content).map_err(|e| DowError::new(e.to_string(), 1))?;

    // Write VERSION (initialize with current branch)
    let version_path = std::path::Path::new("VERSION");
    if !version_path.exists() {
        let branch = crate::core::doc_root::current_branch().unwrap_or_else(|| "main".to_string());
        version::write_branch(&branch, "0.1.0")?;
    }

    // Generate persistent documentation skeleton (docs/ + README.md)
    init_persistent_docs(&args.name, &status_path)?;

    // Write CHANGELOG
    let changelog_path = doc_root_path.join("CHANGELOG.md");
    if !changelog_path.exists() {
        fs::write(&changelog_path, "# Changelog\n").map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // Ensure .gitignore has claim.lock entry
    ensure_gitignore_claim_lock();

    // Detect kiro environment and inject steering
    inject_kiro_steering_if_needed(&args.name);

    let result = InitOutput {
        name: args.name,
        mode: args.mode,
        phase: phase.to_string(),
        doc_root: doc_root_path.to_string_lossy().to_string(),
        version: "0.1.0".to_string(),
    };

    // Silent on success — .dev-doc/ internal operations are not extra info
    let _ = result;
    Ok(0)
}

fn init_persistent_docs(project_name: &str, status_path: &std::path::Path) -> Result<(), DowError> {
    let project_root = doc_root::project_root();
    let docs_dir = project_root.join("docs");
    fs::create_dir_all(&docs_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let readme_path = project_root.join("README.md");
    if !readme_path.exists() {
        let content = format!(
            "# {}\n\n<One-sentence description>\n\n## Quick Start\n\n<Installation and basic usage>\n\n## Documentation\n\n- [Project Structure](docs/structure.md)\n- [Design Decisions](docs/decisions.md)\n- [Usage Guide](docs/usage.md)\n",
            project_name
        );
        fs::write(&readme_path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let files: &[(&str, &str)] = &[
        ("structure.md", "# Project Structure\n\n## Directory Tree\n\n<To be filled>\n\n## Module Responsibilities\n\n<To be filled>\n"),
        ("decisions.md", "# Design Decision Records\n\n## <Decision Title>\n\n- **Date**: YYYY-MM-DD\n- **Decision**: <what>\n- **Rationale**: <why>\n- **Consequences**: <consequence>\n"),
        ("usage.md", "# Usage Guide\n\n## Development Environment\n\n<To be filled>\n\n## Common Tasks\n\n<To be filled>\n"),
    ];

    for (filename, template) in files {
        let path = docs_dir.join(filename);
        if !path.exists() {
            fs::write(&path, template).map_err(|e| DowError::new(e.to_string(), 1))?;
        }
    }

    // Register to STATUS.yaml
    let docs_list = vec![
        "docs/structure.md".to_string(),
        "docs/decisions.md".to_string(),
        "docs/usage.md".to_string(),
    ];
    yaml::set_list(status_path, "docs", &docs_list).map_err(|e| DowError::new(e.to_string(), 1))?;

    Ok(())
}

fn ensure_gitignore_claim_lock() {
    let entry = ".dev-doc/**/claim.lock";
    let gitignore = std::path::Path::new(".gitignore");
    if gitignore.exists() {
        let content = fs::read_to_string(gitignore).unwrap_or_default();
        if !content.lines().any(|l| l.trim() == entry) {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(entry);
            new_content.push('\n');
            let _ = fs::write(gitignore, new_content);
        }
    } else {
        let _ = fs::write(gitignore, format!("{}\n", entry));
    }
}

fn inject_kiro_steering_if_needed(project_name: &str) {
    let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(h) => h,
        Err(_) => return,
    };
    let kiro_dir = std::path::PathBuf::from(&home).join(".kiro");
    if !kiro_dir.is_dir() {
        return;
    }

    let steering_dir = kiro_dir.join("steering");
    let _ = fs::create_dir_all(&steering_dir);

    let steering_file = steering_dir.join("dev-flow.md");
    let content = format!(
        "---\ninclusion: auto\n---\n\n# Dev-Flow Project: {}\n\n\
        This project uses dev-flow for lifecycle management.\n\
        Use dev-flow skills (dev-flow-init, dev-flow-status, dev-flow-task, etc.) to manage workflow.\n\
        Hooks are configured in `.kiro/hooks/` for guard, context injection, and changelog.\n",
        project_name
    );
    let _ = fs::write(&steering_file, content);
}
