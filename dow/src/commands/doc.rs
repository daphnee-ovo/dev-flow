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
            "- [ ] I{}：\n  - severity: P1\n  - location：\n  - description：\n  - reproduce：\n  - fix：\n\n",
            i
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
    r#"# 产品需求文档（PRD）

## 1. 背景与动机

## 2. 目标与非目标
### 目标
### 非目标（明确不做什么）

## 3. 用户画像

## 4. 功能需求
### Must Have
### Should Have
### Could Have
### Won't Have

## 5. 用户流程

## 6. 成功指标

## 7. 约束与假设

## 8. 开放问题
"#.to_string()
}

fn spec_template() -> String {
    r#"# SPEC:

## Goal

## Scope
### In
### Out

## Requirements Trace
| Req | AC | Notes |
| --- | --- | --- |

## Design

## Acceptance
- SPEC-AC-001:
- SPEC-AC-002:

## Risks

## Test Plan

## Self Check
- [ ] 目标清楚
- [ ] 边界清楚
- [ ] 验收可测
- [ ] 与当前 mode 匹配
"#.to_string()
}

fn test_template() -> String {
    r#"# 测试报告

- 执行时间：
- 测试范围：全量 / 指定模块
- 总用例数：
- 通过：
- 失败：

## 失败用例

| 模块 | 用例 | 错误信息 | 关联 issue |
|------|------|----------|-----------|

## 通过模块
"#.to_string()
}

fn brainstorm_template() -> String {
    r#"# 头脑风暴记录 —

**日期**：

## 背景与目的

## 关键决策
| 决策点 | 选择 | 理由 |
|--------|------|------|

## 设计方案

### 架构

### 组件

### 数据流

### 错误处理

## 约束与边界

## 下一步
"#.to_string()
}

fn changelog_template() -> String {
    "# Changelog\n".to_string()
}
