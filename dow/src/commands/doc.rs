// dow/src/commands/
// ├── doc.rs  -- dow doc（文档模板生成 + 文档规范查询）

use crate::cli::DocArgs;
use crate::core::{doc_root, yaml};
use crate::error::DowError;
use crate::output;
use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

// 嵌入 references/.dev-doc/ 的规范文件
const REF_TASK: &str = include_str!("../../../references/.dev-doc/TASK-FILE.md");
const REF_ISSUE: &str = include_str!("../../../references/.dev-doc/ISSUE.md");
const REF_PRD: &str = include_str!("../../../references/.dev-doc/PRD-FILE.md");
const REF_SPEC: &str = include_str!("../../../references/.dev-doc/SPEC-FILE.md");
const REF_TEST: &str = include_str!("../../../references/.dev-doc/TEST.md");
const REF_BRAINSTORM: &str = include_str!("../../../references/.dev-doc/BRAINSTORM-FILE.md");
const REF_CHANGELOG: &str = include_str!("../../../references/.dev-doc/CHANGELOG.md");

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

    // --md / --json：输出文档规范（spec/prd 按当前 mode 过滤）
    if args.md || args.json {
        let mode = get_current_mode();
        return output_spec(&doc_type, args.md, &mode);
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
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);

    let mode = get_current_mode();
    let (path, slots) = match doc_type.as_str() {
        "task" => create_task(&doc_root_path, args.count)?,
        "issue" => create_issue(&doc_root_path, args.count, args.source.as_deref())?,
        "prd" => create_single(&doc_root_path, "PRD.md", prd_template(&mode))?,
        "spec" => create_single(&doc_root_path, "SPEC.md", spec_template(&mode))?,
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

/// 输出文档规范（--md 或 --json），spec/prd 按 mode 过滤展示
fn output_spec(doc_type: &str, as_md: bool, mode: &str) -> Result<i32, DowError> {
    let content = get_reference(doc_type);

    if as_md {
        let filtered = filter_md_by_mode(doc_type, content, mode);
        println!("{}", filtered);
    } else {
        let mut parsed = parse_spec_to_json(doc_type, content);
        filter_json_by_mode(&mut parsed, mode);
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

    // create_command：提示使用 dow doc 创建（task/issue 支持 -n）
    match doc_type {
        "task" => {
            result["create_command"] = json!("dow doc task -n <数量>");
            result["create_hint"] = json!("禁止手动创建 task 文件，必须通过此命令生成模板");
        }
        "issue" => {
            result["create_command"] = json!("dow doc issue -n <数量> [--source test|devtest|audit|other]");
            result["create_hint"] = json!("禁止手动创建 issue 文件，必须通过此命令生成模板");
        }
        "prd" | "spec" | "test" | "brainstorm" | "changelog" => {
            result["create_command"] = json!(format!("dow doc {}", doc_type));
            result["create_hint"] = json!(format!("禁止手动创建 {}.md，必须通过此命令生成", doc_type.to_uppercase()));
        }
        _ => {}
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
            "- [ ] TASK-T{:03}: \n  - type: feat\n  - priority: P1\n  - refs: \n  - files:\n      create: []\n      modify: []\n      test: []\n  - depends_on: []\n  - parallel: false\n  - complexity: S\n  - done_when:\n      - \n\n",
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

fn prd_template_inner(mode: &str, with_hints: bool) -> String {
    let sections = prd_sections_for_mode(mode);
    if sections.is_empty() {
        return format!("# 产品需求文档（PRD）\n\n> {} 模式跳过 PRD 阶段。\n", mode);
    }

    let mut out = String::from("# 产品需求文档（PRD）\n\n");
    for (i, sec) in sections.iter().enumerate() {
        out.push_str(&format!("## {}. {}\n", i + 1, sec));
        if *sec == "功能需求" {
            out.push_str("### Must Have\n### Should Have\n### Could Have\n### Won't Have\n");
        }
        if *sec == "目标与非目标" {
            out.push_str("### 目标\n### 非目标（明确不做什么）\n");
        }
        if with_hints {
            match *sec {
                "背景与动机" => { out.push_str("<为什么要做这件事>\n"); }
                "用户画像" => { out.push_str("<目标用户是谁>\n"); }
                "用户流程" => { out.push_str("<用户如何使用>\n"); }
                "成功指标" => { out.push_str("<如何衡量成功>\n"); }
                "约束与假设" => { out.push_str("<前提条件和限制>\n"); }
                "开放问题" => { out.push_str("<待确认事项>\n"); }
                _ => {}
            }
        }
        out.push('\n');
    }
    out
}

fn prd_template(mode: &str) -> String {
    prd_template_inner(mode, false)
}

fn prd_template_with_hints(mode: &str) -> String {
    prd_template_inner(mode, true)
}

/// 生成 SPEC 模板。with_hints=true 时带占位提示（用于 --md 展示），false 时干净空白（用于创建文件）
fn spec_template_inner(mode: &str, with_hints: bool) -> String {
    let sections = spec_sections_for_mode(mode);
    let title_hint = if with_hints { " <主题>" } else { "" };
    let mut out = format!("# SPEC:{}\n\n", title_hint);

    for sec in &sections {
        out.push_str(&format!("## {}\n", sec));
        match *sec {
            "Goal" => {
                if with_hints { out.push_str("<目标>\n"); }
            }
            "Scope" => {
                out.push_str("### In\n### Out\n");
            }
            "Out of scope" => {
                if with_hints { out.push_str("<明确不做的边界>\n"); }
            }
            "Requirements Trace" => {
                out.push_str("| Req | AC | Notes |\n| --- | --- | --- |\n");
                if with_hints {
                    out.push_str("| PRD-FR-001 或 user-request | SPEC-AC-001 | ADDED / MODIFIED / REMOVED |\n");
                }
            }
            "Design" => {
                if with_hints { out.push_str("<必要方案。能短就短。>\n"); }
            }
            "Acceptance" => {
                if with_hints {
                    out.push_str("- SPEC-AC-001: <可测验收>\n- SPEC-AC-002: <可测验收>\n");
                } else {
                    out.push_str("- SPEC-AC-001:\n- SPEC-AC-002:\n");
                }
            }
            "Risks" => {
                if with_hints { out.push_str("- <风险和回退>\n"); }
            }
            "Test Plan" => {
                if with_hints { out.push_str("- <最小验证方式>\n"); } else { out.push_str("- \n"); }
            }
            "Smoke Test" => {
                if with_hints { out.push_str("- <冒烟测试>\n"); } else { out.push_str("- \n"); }
            }
            _ => {}
        }
        out.push('\n');
    }

    // Self Check 始终包含
    out.push_str("## Self Check\n");
    out.push_str("- [ ] 目标清楚\n");
    if sections.contains(&"Scope") || sections.contains(&"Out of scope") {
        out.push_str("- [ ] 边界清楚\n");
    }
    if sections.contains(&"Acceptance") {
        out.push_str("- [ ] 验收可测\n");
    }
    out.push_str("- [ ] 与当前 mode 匹配\n");
    out
}

fn spec_template(mode: &str) -> String {
    spec_template_inner(mode, false)
}

fn spec_template_with_hints(mode: &str) -> String {
    spec_template_inner(mode, true)
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

// === Mode 感知功能 ===

/// 读取当前项目 mode（从 STATUS.yaml），fallback 为 "full"
fn get_current_mode() -> String {
    let doc_root_path = doc_root::resolve(crate::core::DOC_DIR);
    let status_file = doc_root_path.join("STATUS.yaml");
    if !status_file.exists() {
        return "full".to_string();
    }
    let mode = yaml::get(&status_file, "mode").ok().flatten().unwrap_or_default();
    // audit/xxx → 提取原始 mode
    if let Some(orig) = mode.strip_prefix("audit/") {
        orig.to_string()
    } else if mode.is_empty() {
        "full".to_string()
    } else {
        mode
    }
}

/// SPEC 各章节在不同 mode 下是否必需
fn spec_sections_for_mode(mode: &str) -> Vec<&'static str> {
    match mode {
        "fast" => vec!["Goal", "Acceptance", "Test Plan"],
        "mvp" => vec!["Goal", "Out of scope", "Smoke Test"],
        "quick" => vec!["Goal", "Scope", "Design", "Acceptance", "Test Plan"],
        _ => vec!["Goal", "Scope", "Requirements Trace", "Design", "Acceptance", "Risks", "Test Plan"],
    }
}

/// PRD 各章节在不同 mode 下是否必需
fn prd_sections_for_mode(mode: &str) -> Vec<&'static str> {
    match mode {
        "fast" | "mvp" => vec![],  // fast/mvp 跳过 PRD
        "quick" => vec!["背景与动机", "目标与非目标", "功能需求", "成功指标", "开放问题"],
        _ => vec!["背景与动机", "目标与非目标", "用户画像", "功能需求", "用户流程", "成功指标", "约束与假设", "开放问题"],
    }
}

/// 按 mode 过滤 --md 输出（处理 spec/prd 的「按 mode 的必需章节」和模板段）
fn filter_md_by_mode(doc_type: &str, content: &str, mode: &str) -> String {
    if doc_type != "spec" && doc_type != "prd" {
        return content.to_string();
    }

    let mut result = Vec::new();
    let mut skip_mode_table = false;
    let mut skip_template_block = false;
    let mut in_code_fence = false;  // 追踪代码块内外

    for line in content.lines() {
        // 检测「按 mode 的必需章节」段落 → 替换为当前 mode 列表
        if !in_code_fence && line.starts_with("## 按 mode 的必需章节") {
            let sections = if doc_type == "spec" {
                spec_sections_for_mode(mode)
            } else {
                prd_sections_for_mode(mode)
            };
            if sections.is_empty() {
                // 当前 mode 跳过此阶段，展示 full 模式作参考
                result.push(format!("## 按 mode 的必需章节（当前 {} 模式跳过此阶段）", mode));
                result.push(String::new());
                result.push("以下展示 full 模式的要求：".to_string());
                result.push(String::new());
                let full_sections = if doc_type == "spec" {
                    spec_sections_for_mode("full")
                } else {
                    prd_sections_for_mode("full")
                };
                for s in &full_sections {
                    result.push(format!("- {}", s));
                }
            } else {
                result.push(format!("## 当前 mode（{}）的必需章节", mode));
                result.push(String::new());
                for s in &sections {
                    result.push(format!("- {}", s));
                }
            }
            result.push(String::new());
            skip_mode_table = true;
            continue;
        }

        // 跳过原始 mode 表格和降级规则直到下一个真正的 ## 段
        if skip_mode_table {
            if !in_code_fence && line.starts_with("## ") {
                skip_mode_table = false;
            } else {
                // 跟踪代码块
                if line.trim().starts_with("```") {
                    in_code_fence = !in_code_fence;
                }
                continue;
            }
        }

        // 检测模板段 → 替换为当前 mode 的动态模板
        if !in_code_fence && line.starts_with("## 模板") {
            let sections = if doc_type == "spec" {
                spec_sections_for_mode(mode)
            } else {
                prd_sections_for_mode(mode)
            };
            if sections.is_empty() {
                // 跳过的阶段：标注并展示 full 模式模板作参考
                result.push(format!("## 模板（当前 {} 模式跳过此阶段，以下为 full 模式参考）", mode));
            } else {
                result.push("## 模板".to_string());
            }
            result.push(String::new());
            result.push("```markdown".to_string());
            let effective_mode = if sections.is_empty() { "full" } else { mode };
            let template = if doc_type == "spec" {
                spec_template_with_hints(effective_mode)
            } else {
                prd_template_with_hints(effective_mode)
            };
            result.push(template.trim_end().to_string());
            result.push("```".to_string());
            result.push(String::new());
            skip_template_block = true;
            continue;
        }

        // 跳过原始模板代码块（需追踪 ``` 结束）
        if skip_template_block {
            if line.trim().starts_with("```") {
                in_code_fence = !in_code_fence;
                // 代码块结束后等待下一个 ## 段
                if !in_code_fence {
                    continue;
                }
            }
            if in_code_fence {
                continue;
            }
            // 代码块已结束，遇到下一个 ## 段则恢复正常
            if line.starts_with("## ") {
                skip_template_block = false;
                result.push(line.to_string());
            }
            continue;
        }

        // 正常行：追踪代码块状态
        if line.trim().starts_with("```") {
            in_code_fence = !in_code_fence;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

/// 按 mode 过滤 --json 输出中的 mode_requirements 和 template
fn filter_json_by_mode(parsed: &mut Value, mode: &str) {
    let doc_type = parsed.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // mode_requirements：只保留当前 mode 有标记的行
    if let Some(arr) = parsed.get("mode_requirements").and_then(|v| v.as_array()) {
        let filtered: Vec<Value> = arr.iter().filter(|row| {
            if let Some(val) = row.get(mode).and_then(|v| v.as_str()) {
                val == "✓" || !val.contains('—')
            } else {
                true
            }
        }).cloned().collect();
        parsed["mode_requirements"] = json!(filtered);
    }

    // template：替换为当前 mode 的动态模板（带占位提示）
    if doc_type == "spec" || doc_type == "prd" {
        let template = if doc_type == "spec" {
            spec_template_with_hints(mode)
        } else {
            prd_template_with_hints(mode)
        };
        parsed["template"] = json!(template.trim_end());
    }

    parsed["current_mode"] = json!(mode);
}
