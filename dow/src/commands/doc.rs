// dow/src/commands/
// ├── doc.rs  -- dow doc（文档模板生成 + 文档规范查询）

use crate::cli::DocArgs;
use crate::core::doc_root;
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

// 嵌入 references/dev-doc/ 的规范文件
const REF_TASK: &str = include_str!("../../../references/dev-doc/TASK-FILE.md");
const REF_ISSUE: &str = include_str!("../../../references/dev-doc/ISSUE.md");
const REF_PRD: &str = include_str!("../../../references/dev-doc/PRD-FILE.md");
const REF_SPEC: &str = include_str!("../../../references/dev-doc/SPEC-FILE.md");
const REF_TEST: &str = include_str!("../../../references/dev-doc/TEST.md");
const REF_BRAINSTORM: &str = include_str!("../../../references/dev-doc/BRAINSTORM-FILE.md");
const REF_CHANGELOG: &str = include_str!("../../../references/dev-doc/CHANGELOG.md");

#[derive(Serialize)]
struct DocOutput {
    created: String,
    #[serde(rename = "type")]
    doc_type: String,
    slots: u32,
}

/// 合法的文档类型
const VALID_TYPES: &[&str] = &["task", "issue", "prd", "spec", "test", "brainstorm", "changelog"];

/// 合法的 issue 来源
const VALID_SOURCES: &[&str] = &["test", "devtest", "other", "audit"];

pub fn run(args: DocArgs, human: bool) -> Result<i32, DowError> {
    let doc_type = args.doc_type.to_lowercase();

    if !VALID_TYPES.contains(&doc_type.as_str()) {
        return Err(DowError::new(
            format!(
                "未知文档类型：{}（可选：{}）",
                doc_type,
                VALID_TYPES.join("/")
            ),
            1,
        ));
    }

    // --md / --json：输出文档规范
    if args.md || args.json {
        return output_spec(&doc_type, args.md);
    }

    // 校验 --source（仅 issue 类型使用）
    if let Some(ref src) = args.source {
        if !VALID_SOURCES.contains(&src.as_str()) {
            return Err(DowError::new(
                format!(
                    "无效的 issue 来源：{}（可选：{}）",
                    src,
                    VALID_SOURCES.join("/")
                ),
                1,
            ));
        }
    }

    // 默认：创建模板文件
    let doc_root_path = doc_root::resolve("dev-doc");

    let (path, slots) = match doc_type.as_str() {
        "task" => create_task(&doc_root_path, args.count)?,
        "issue" => create_issue(&doc_root_path, args.count, args.source.as_deref())?,
        "prd" => create_single(&doc_root_path, "PRD.md", prd_template())?,
        "spec" => create_single(&doc_root_path, "SPEC.md", spec_template())?,
        "test" => create_single(&doc_root_path, "TEST.md", test_template())?,
        "brainstorm" => create_single(&doc_root_path, "BRAINSTORM.md", brainstorm_template())?,
        "changelog" => create_single(&doc_root_path, "CHANGELOG.md", changelog_template())?,
        _ => unreachable!(),
    };

    let result = DocOutput {
        created: path,
        doc_type,
        slots,
    };

    if human {
        println!("[dev-flow] 文档已创建：{}", result.created);
        match result.doc_type.as_str() {
            "task" | "issue" => {
                println!(
                    "  提示：-n <数量> 可指定模板中的条目数，如 dow doc {} -n 5",
                    result.doc_type
                );
            }
            _ => {}
        }
        println!(
            "  提示：使用 dow doc {} --md 或 dow doc {} --json 查看文档格式规范",
            result.doc_type, result.doc_type
        );
    } else {
        output::print_json(&result);
    }

    Ok(0)
}

/// 输出文档规范（--md 或 --json）
fn output_spec(doc_type: &str, as_md: bool) -> Result<i32, DowError> {
    let content = get_reference(doc_type);

    if as_md {
        println!("{}", content);
    } else {
        let parsed = parse_spec_to_json(doc_type, content);
        println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
    }

    Ok(0)
}

/// 获取对应类型的 reference 文档内容
fn get_reference(doc_type: &str) -> &'static str {
    match doc_type {
        "task" => REF_TASK,
        "issue" => REF_ISSUE,
        "prd" => REF_PRD,
        "spec" => REF_SPEC,
        "test" => REF_TEST,
        "brainstorm" => REF_BRAINSTORM,
        "changelog" => REF_CHANGELOG,
        _ => unreachable!(),
    }
}

/// 将 markdown 规范解析为结构化 JSON
fn parse_spec_to_json(doc_type: &str, content: &str) -> Value {
    let mut result = json!({
        "type": doc_type,
    });

    // 提取标题
    if let Some(title_line) = content.lines().find(|l| l.starts_with("# ")) {
        result["title"] = json!(title_line.trim_start_matches("# ").trim());
    }

    // 提取路径（## 路径 段落下的内容）
    if let Some(path) = extract_section_first_line(content, "路径") {
        result["path"] = json!(path);
    }

    // 提取模板（```markdown ... ``` 代码块）
    if let Some(template) = extract_code_block(content, "markdown") {
        result["template"] = json!(template);
    }

    // 提取字段说明表格
    if let Some(fields) = extract_table(content, "字段说明") {
        result["fields"] = fields;
    }

    // 提取 sections 列表
    let sections = extract_h2_sections(content);
    if !sections.is_empty() {
        result["sections"] = json!(sections);
    }

    // 提取规则/说明列表
    let mut rules = Vec::new();
    for section_name in &["完成规则", "命名规则", "说明", "追加规则", "注意事项"] {
        if let Some(items) = extract_bullet_list(content, section_name) {
            for item in items {
                rules.push(json!({
                    "category": section_name,
                    "rule": item,
                }));
            }
        }
    }
    if !rules.is_empty() {
        result["rules"] = json!(rules);
    }

    // 按 mode 的必需章节（SPEC/PRD 特有）
    if let Some(mode_table) = extract_table(content, "按 mode 的必需章节") {
        result["mode_requirements"] = mode_table;
    }

    // 优先级/severity 定义
    if let Some(prio_table) = extract_table(content, "Priority 定义") {
        result["priority_definitions"] = prio_table;
    }
    if let Some(complexity_table) = extract_table(content, "Complexity 定义") {
        result["complexity_definitions"] = complexity_table;
    }

    result
}

/// 提取某个 ## 段落的第一个非空行
fn extract_section_first_line(content: &str, heading: &str) -> Option<String> {
    let marker = format!("## {}", heading);
    let mut in_section = false;
    for line in content.lines() {
        if line.starts_with(&marker) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// 提取指定语言的代码块
fn extract_code_block(content: &str, lang: &str) -> Option<String> {
    let opener = format!("```{}", lang);
    let mut in_block = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if !in_block && line.trim().starts_with(&opener) {
            in_block = true;
            continue;
        }
        if in_block {
            if line.trim() == "```" {
                break;
            }
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// 提取某段落下的表格为 JSON 数组
fn extract_table(content: &str, heading: &str) -> Option<Value> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;

    // 查找标题（支持 ## 和 ### 开头）
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed == heading {
            start = Some(i + 1);
            break;
        }
    }

    let start = start?;

    // 寻找表格（| 开头的行）
    let mut table_lines = Vec::new();
    let mut found_table = false;
    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.starts_with('|') {
            found_table = true;
            table_lines.push(trimmed);
        } else if found_table {
            break;
        } else if trimmed.starts_with('#') {
            break;
        }
    }

    if table_lines.len() < 3 {
        return None;
    }

    // 解析表头
    let headers: Vec<String> = table_lines[0]
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // 跳过分隔行（第二行），解析数据行
    let mut rows = Vec::new();
    for row_line in &table_lines[2..] {
        let cells: Vec<&str> = row_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect();

        let mut row = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = cells.get(i).unwrap_or(&"");
            row.insert(header.clone(), json!(val));
        }
        rows.push(Value::Object(row));
    }

    Some(json!(rows))
}

/// 提取所有 ## 段落标题
fn extract_h2_sections(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| l.starts_with("## "))
        .map(|l| l.trim_start_matches("## ").trim().to_string())
        .collect()
}

/// 提取某段落下的列表项
fn extract_bullet_list(content: &str, heading: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed == heading {
            start = Some(i + 1);
            break;
        }
    }

    let start = start?;
    let mut items = Vec::new();

    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            break;
        }
        if let Some(item) = trimmed.strip_prefix("- ") {
            items.push(item.to_string());
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

// === 模板创建功能（保持原有逻辑） ===

fn create_task(doc_root: &Path, count: u32) -> Result<(String, u32), DowError> {
    let task_dir = doc_root.join("task");
    fs::create_dir_all(&task_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let seq = next_seq(&task_dir, &format!("task_{}", today));
    let filename = format!("task_{}_{}.md", today, seq);
    let path = task_dir.join(&filename);

    let mut content = format!("---\ntitle: TASK - \nnums: {}\n---\n\n", count);

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] TASK-T{:03}: \n  - priority: P1\n  - refs: \n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - \n\n",
            i
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_issue(
    doc_root: &Path,
    count: u32,
    source: Option<&str>,
) -> Result<(String, u32), DowError> {
    let issue_dir = doc_root.join("issue");
    fs::create_dir_all(&issue_dir).map_err(|e| DowError::new(e.to_string(), 1))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let src = source.unwrap_or("other");
    let seq = next_seq(&issue_dir, &format!("issue_{}_{}", src, today));
    let filename = format!("issue_{}_{}_{}.md", src, today, seq);
    let path = issue_dir.join(&filename);

    let mut content = format!("---\nsource: {}\nnums: {}\n---\n\n", src, count);

    for i in 1..=count {
        content.push_str(&format!(
            "- [ ] ISSUE-I{:03}：\n  - severity: P1\n  - location：\n  - description：\n  - reproduce：\n  - fix：\n\n",
            i
        ));
    }

    fs::write(&path, content).map_err(|e| DowError::new(e.to_string(), 1))?;
    Ok((path.to_string_lossy().to_string(), count))
}

fn create_single(
    doc_root: &Path,
    filename: &str,
    template: String,
) -> Result<(String, u32), DowError> {
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
"#
    .to_string()
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
"#
    .to_string()
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
"#
    .to_string()
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
"#
    .to_string()
}

fn changelog_template() -> String {
    "# Changelog\n".to_string()
}
