// dow/src/core/
// ├── item_id.rs  -- Unified task/issue ID parsing and normalization

/// Item 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Task,
    Issue,
}

/// 解析后的 item ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemId {
    pub kind: ItemKind,
    pub num: u32,
}

impl ItemId {
    /// 短序号: "T001" / "I003"
    pub fn short(&self) -> String {
        match self.kind {
            ItemKind::Task => format!("T{:03}", self.num),
            ItemKind::Issue => format!("I{:03}", self.num),
        }
    }

    /// 长序号: "TASK-T001" / "ISSUE-I003"
    pub fn full(&self) -> String {
        match self.kind {
            ItemKind::Task => format!("TASK-T{:03}", self.num),
            ItemKind::Issue => format!("ISSUE-I{:03}", self.num),
        }
    }

    /// 序号数字
    pub fn num(&self) -> u32 {
        self.num
    }
}

/// 解析任意格式的 item ID。
/// 支持: "TASK-T001" / "T001" / "T1" / "ISSUE-I003" / "I003" / "I3"
pub fn parse(input: &str) -> Option<ItemId> {
    let s = input.trim();

    if let Some(rest) = s.strip_prefix("TASK-T") {
        let num: u32 = rest.parse().ok()?;
        return Some(ItemId {
            kind: ItemKind::Task,
            num,
        });
    }
    if let Some(rest) = s.strip_prefix("ISSUE-I") {
        let num: u32 = rest.parse().ok()?;
        return Some(ItemId {
            kind: ItemKind::Issue,
            num,
        });
    }
    if let Some(rest) = s.strip_prefix('T') {
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            let num: u32 = rest.parse().ok()?;
            return Some(ItemId {
                kind: ItemKind::Task,
                num,
            });
        }
    }
    if let Some(rest) = s.strip_prefix('I') {
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            let num: u32 = rest.parse().ok()?;
            return Some(ItemId {
                kind: ItemKind::Issue,
                num,
            });
        }
    }
    None
}

/// 归一化为短序号。输入任意合法格式，输出 "T001" / "I003"。
/// 非法输入原样返回。
pub fn normalize_short(input: &str) -> String {
    match parse(input) {
        Some(id) => id.short(),
        None => input.to_string(),
    }
}

/// 归一化为长序号。输入任意合法格式，输出 "TASK-T001" / "ISSUE-I003"。
/// 非法输入原样返回。
pub fn normalize_full(input: &str) -> String {
    match parse(input) {
        Some(id) => id.full(),
        None => input.to_string(),
    }
}

/// 从 checklist 行提取 item ID（如 "- [ ] TASK-T001: xxx" → Some(ItemId)）
pub fn extract_from_line(line: &str) -> Option<ItemId> {
    let trimmed = line.trim();
    let after = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        rest
    } else {
        return None;
    };
    let id_part = after.split([':', '：']).next()?.trim();
    parse(id_part)
}

/// 判断 checklist 行是否为未完成项
pub fn is_undone_line(line: &str) -> bool {
    line.trim().starts_with("- [ ] ")
}

/// 判断 checklist 行是否为已完成项
pub fn is_done_line(line: &str) -> bool {
    line.trim().starts_with("- [x] ")
}
