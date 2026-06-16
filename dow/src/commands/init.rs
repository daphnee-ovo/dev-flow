// dow/src/commands/
// ├── init.rs  -- dow init（初始化 dev-flow 工作流管理）

use crate::cli::InitArgs;
use crate::core::{doc_root, version, yaml};
use crate::error::DowError;
use crate::output;
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

pub fn run(args: InitArgs, human: bool) -> Result<i32, DowError> {
    let valid_modes = ["full", "quick", "fast", "mvp"];
    if !valid_modes.contains(&args.mode.as_str()) {
        return Err(DowError::new(
            format!("无效模式：{}（可选：full/quick/fast/mvp）", args.mode),
            1,
        ));
    }

    let base_dir = std::path::Path::new(crate::core::DOC_DIR);
    fs::create_dir_all(base_dir)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    // 解析多分支模式路径：.dev-doc/<branch>/
    let branch = crate::core::doc_root::current_branch()
        .unwrap_or_else(|| "main".to_string());
    let doc_root_path = base_dir.join(&branch);
    fs::create_dir_all(&doc_root_path)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    // 创建目录结构（archive 已迁移到 SQLite，不再创建目录）
    for dir in &["issue", "task"] {
        fs::create_dir_all(doc_root_path.join(dir))
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }
    fs::create_dir_all("tests")
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    // tmp 目录：如果已有 temp 就不创建 tmp
    if !std::path::Path::new("temp").is_dir() {
        fs::create_dir_all("tmp")
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 确定起始阶段
    let phase = match args.mode.as_str() {
        "full" => "PRD",
        "quick" | "mvp" => "SPEC",
        "fast" => "TASK",
        _ => "DEV",
    };

    // 写入 STATUS.yaml
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let status_content = format!(
        "name: {}\nphase: {}\nmode: {}\nexec_mode: step\nupdated: \"{}\"\nstarted: \"{}\"\n",
        args.name, phase, args.mode, now, now
    );
    let status_path = doc_root_path.join("STATUS.yaml");
    if status_path.exists() {
        return Err(DowError::new(
            "STATUS.yaml 已存在，如需重新初始化请先删除",
            1,
        ));
    }
    fs::write(&status_path, &status_content)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    // 写入 VERSION（以当前分支初始化）
    let version_path = std::path::Path::new("VERSION");
    if !version_path.exists() {
        let branch = crate::core::doc_root::current_branch()
            .unwrap_or_else(|| "main".to_string());
        version::write_branch(&branch, "0.1.0")?;
    }

    // 生成持久化文档骨架（docs/ + README.md）
    init_persistent_docs(&args.name, &status_path)?;

    // 写入 CHANGELOG
    let changelog_path = doc_root_path.join("CHANGELOG.md");
    if !changelog_path.exists() {
        fs::write(&changelog_path, "# Changelog\n")
            .map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    // 检测 kiro 环境并注入 steering
    inject_kiro_steering_if_needed(&args.name);

    let result = InitOutput {
        name: args.name,
        mode: args.mode,
        phase: phase.to_string(),
        doc_root: doc_root_path.to_string_lossy().to_string(),
        version: "0.1.0".to_string(),
    };

    if human {
        println!("[dev-flow] 初始化完成");
        println!("━━━━━━━━━━━━━━━━━━━━━━");
        println!("项目名称：{}", result.name);
        println!("开发模式：{}", result.mode);
        println!("当前阶段：{}", result.phase);
        println!("版本：v{}", result.version);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn init_persistent_docs(project_name: &str, status_path: &std::path::Path) -> Result<(), DowError> {
    let project_root = doc_root::project_root();
    let docs_dir = project_root.join("docs");
    fs::create_dir_all(&docs_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let readme_path = project_root.join("README.md");
    if !readme_path.exists() {
        let content = format!(
            "# {}\n\n<一句话描述>\n\n## 快速开始\n\n<安装和基本使用>\n\n## 文档\n\n- [项目结构](docs/structure.md)\n- [设计决策](docs/decisions.md)\n- [使用指南](docs/usage.md)\n",
            project_name
        );
        fs::write(&readme_path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    }

    let files: &[(&str, &str)] = &[
        ("structure.md", "# 项目结构\n\n## 目录树\n\n<待填充>\n\n## 模块职责\n\n<待填充>\n"),
        ("decisions.md", "# 设计决策记录\n\n## <决策标题>\n\n- **日期**：YYYY-MM-DD\n- **决策**：<what>\n- **理由**：<why>\n- **后果**：<consequence>\n"),
        ("usage.md", "# 使用指南\n\n## 开发环境\n\n<待填充>\n\n## 常见任务\n\n<待填充>\n"),
    ];

    for (filename, template) in files {
        let path = docs_dir.join(filename);
        if !path.exists() {
            fs::write(&path, template).map_err(|e| DowError::new(e.to_string(), 1))?;
        }
    }

    // 注册到 STATUS.yaml
    let docs_list = vec![
        "docs/structure.md".to_string(),
        "docs/decisions.md".to_string(),
        "docs/usage.md".to_string(),
    ];
    yaml::set_list(status_path, "docs", &docs_list)
        .map_err(|e| DowError::new(e.to_string(), 1))?;

    Ok(())
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
