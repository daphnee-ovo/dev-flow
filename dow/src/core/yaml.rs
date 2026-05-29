// dow/src/lib/
// ├── yaml.rs  -- STATUS.yaml 轻量读写（不依赖 YAML 库）

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// 读取 STATUS.yaml 为有序键值对
pub fn read(path: &Path) -> std::io::Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)?;
    Ok(parse(&content))
}

/// 解析 YAML 格式的简单键值对
pub fn parse(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        if let Some((key, value)) = parse_line(line) {
            map.insert(key, value);
        }
    }
    map
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let colon_pos = trimmed.find(':')?;
    let key = trimmed[..colon_pos].trim().to_string();
    let value = trimmed[colon_pos + 1..].trim().to_string();
    Some((key, value))
}

/// 获取指定 key 的值
pub fn get(path: &Path, key: &str) -> std::io::Result<Option<String>> {
    let map = read(path)?;
    Ok(map.get(key).cloned())
}

/// 设置指定 key 的值（保持文件其他行不变）
pub fn set(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
    let content = fs::read_to_string(path)?;
    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            if let Some((k, _)) = parse_line(line) {
                if k == key {
                    found = true;
                    return format!("{}: {}", key, value);
                }
            }
            line.to_string()
        })
        .collect();

    if !found {
        // goals/exec_mode 等字段插入到 updated/started 之前（保持时间戳在末尾）
        let insert_pos = lines.iter().position(|l| {
            l.starts_with("updated:") || l.starts_with("started:")
        });
        if let Some(pos) = insert_pos {
            lines.insert(pos, format!("{}: {}", key, value));
        } else {
            lines.push(format!("{}: {}", key, value));
        }
    }

    // 确保 updated/started 始终在最后
    let mut time_lines = Vec::new();
    lines.retain(|l| {
        if l.starts_with("updated:") || l.starts_with("started:") {
            time_lines.push(l.clone());
            false
        } else {
            true
        }
    });
    lines.extend(time_lines);

    // 确保文件末尾有换行
    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    fs::write(path, output)
}

/// 更新 updated 时间戳
pub fn touch_updated(path: &Path) -> std::io::Result<()> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    set(path, "updated", &now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let content = "name: dev-flow\nphase: DEV\nmode: quick\n";
        let map = parse(content);
        assert_eq!(map.get("name"), Some(&"dev-flow".to_string()));
        assert_eq!(map.get("phase"), Some(&"DEV".to_string()));
        assert_eq!(map.get("mode"), Some(&"quick".to_string()));
    }

    #[test]
    fn test_parse_with_spaces_in_value() {
        let content = "updated: 2026-05-26 16:58\n";
        let map = parse(content);
        assert_eq!(map.get("updated"), Some(&"2026-05-26 16:58".to_string()));
    }

    #[test]
    fn test_parse_empty_lines_and_comments() {
        let content = "# comment\nname: test\n\nphase: DEV\n";
        let map = parse(content);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("name"), Some(&"test".to_string()));
    }
}
