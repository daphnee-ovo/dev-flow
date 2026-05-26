// dow/src/commands/
// ├── doc.rs  -- dow doc（文档模板生成）

use crate::cli::DocArgs;
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct DocOutput {
    created: String,
    #[serde(rename = "type")]
    doc_type: String,
    slots: u32,
}

pub fn run(args: DocArgs, human: bool) -> Result<i32, DowError> {
    let doc_type = determine_type(&args)?;
    let doc_root_path = doc_root::resolve("dev-doc");

    let (path, slots) = match doc_type.as_str() {
        "task" => create_task(&doc_root_path, args.count)?,
        "issue" => create_issue(&doc_root_path, args.count, args.source.as_deref())?,
        "prd" => create_single(&doc_root_path, "PRD.md", prd_template())?,
        "spec" => create_single(&doc_root_path, "SPEC.md", spec_template())?,
        "test" => create_single(&doc_root_path, "TEST.md", test_template())?,
        "brainstorm" => create_single(&doc_root_path, "BRAINSTORM.md", brainstorm_template())?,
        "changelog" => create_single(&doc_root_path, "CHANGELOG.md", changelog_template())?,
        _ => return Err(DowError::new(format!("未知文档类型：{}", doc_type), 1)),
    };

    let result = DocOutput {
        created: path,
        doc_type,
        slots,
    };

    if human {
        println!("[dev-flow] 文档已创建：{}", result.created);
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

fn determine_type(args: &DocArgs) -> Result<String, DowError> {
    if args.task { return Ok("task".to_string()); }
    if args.issue { return Ok("issue".to_string()); }
    if args.prd { return Ok("prd".to_string()); }
    if args.spec { return Ok("spec".to_string()); }
    if args.test { return Ok("test".to_string()); }
    if args.brainstorm { return Ok("brainstorm".to_string()); }
    if args.changelog { return Ok("changelog".to_string()); }
    Err(DowError::new("请指定文档类型：--task/--issue/--prd/--spec/--test/--brainstorm/--changelog", 1))
}

fn create_task(doc_root: &Path, count: u32) -> Result<(String, u32), DowError> {
    let task_dir = doc_root.join("task");
    fs::create_dir_all(&task_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let seq = next_seq(&task_dir, &format!("task_{}", today));
    let filename = format!("task_{}_{}.md", today, seq);
    let path = task_dir.join(&filename);

    let mut content = format!(
        "---\ntitle: TASK - \nnums: {}\n---\n\n",
        count
    );

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] TASK-T{:03}: \n  - priority: P1\n  - refs: \n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - \n\n",
            i
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_issue(doc_root: &Path, count: u32, source: Option<&str>) -> Result<(String, u32), DowError> {
    let issue_dir = doc_root.join("issue");
    fs::create_dir_all(&issue_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let src = source.unwrap_or("other");
    let seq = next_seq(&issue_dir, &format!("issue_{}_{}", src, today));
    let filename = format!("issue_{}_{}_{}.md", src, today, seq);
    let path = issue_dir.join(&filename);

    let mut content = format!(
        "---\nsource: {}\nnums: {}\n---\n\n",
        src, count
    );

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] ISSUE-I{:03}: \n  - severity: P1\n  - source: {}\n  - location: \n  - current: \n  - expected: \n  - reproduce: \n  - root_cause:\n  - fix:\n  - close_when: \n\n",
            i, src
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_single(doc_root: &Path, filename: &str, template: String) -> Result<(String, u32), DowError> {
    let path = doc_root.join(filename);
    if path.exists() {
        return Err(DowError::new(
            format!("{} 已存在，不覆盖", path.display()),
            1,
        ));
    }
    fs::write(&path, template).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), 1))
}

fn next_seq(dir: &Path, prefix: &str) -> u32 {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(prefix) {
                // 提取最后的数字部分
                if let Some(num_str) = name.strip_suffix(".md") {
                    if let Some(last) = num_str.rsplit('_').next() {
                        if let Ok(n) = last.parse::<u32>() {
                            max = max.max(n);
                        }
                    }
                }
            }
        }
    }
    max + 1
}

fn prd_template() -> String {
    r#"# PRD -

## 1. 背景与动机

## 2. 目标与非目标

### 目标

### 非目标

## 3. 用户故事

## 4. 功能需求

### Must Have

### Should Have

### Won't Have

## 5. 约束与假设

## 6. 成功指标
"#.to_string()
}

fn spec_template() -> String {
    r#"# SPEC -

## 1. 概述

## 2. 架构设计

## 3. 技术选型

| 选项 | 选择 | 理由 |
|------|------|------|

## 4. 数据模型

## 5. 验收契约

| ID | 验收条件 | 验证方式 |
|----|----------|----------|
| SPEC-AC-001 | | |
"#.to_string()
}

fn test_template() -> String {
    r#"# TEST - 测试报告

## 测试环境

## 测试用例

| ID | 用例 | 预期 | 实际 | 状态 |
|----|------|------|------|------|

## 测试结论
"#.to_string()
}

fn brainstorm_template() -> String {
    r#"# BRAINSTORM -

## 核心问题

## 探索方向

## 约束条件

## 初步想法
"#.to_string()
}

fn changelog_template() -> String {
    "# Changelog\n".to_string()
}
