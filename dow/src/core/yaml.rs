// dow/src/core/
// ├── yaml.rs  -- STATUS.yaml lightweight read/write (no YAML library dependency)
//
// Related Docs:
// - [STATUS Specification](../../../references/.dev-doc/STATUS.md)

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Read STATUS.yaml as ordered key-value pairs
pub fn read(path: &Path) -> std::io::Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)?;
    Ok(parse(&content))
}

/// Parse simple key-value pairs in YAML format
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

/// Get value for specified key
pub fn get(path: &Path, key: &str) -> std::io::Result<Option<String>> {
    let map = read(path)?;
    Ok(map.get(key).cloned())
}

/// Set value for specified key (keep other lines in file unchanged)
/// Delete line when optional field value is empty; required fields cannot be set empty
pub fn set(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
    let content = fs::read_to_string(path)?;
    let optional_fields = ["goals_minor", "goals_major", "exec_mode"];
    let is_delete = value.is_empty() && optional_fields.contains(&key);
    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .filter_map(|line| {
            if let Some((k, _)) = parse_line(line) {
                if k == key {
                    found = true;
                    if is_delete {
                        return None;
                    }
                    return Some(format!("{}: {}", key, value));
                }
            }
            Some(line.to_string())
        })
        .collect();

    if !found && !is_delete {
        // Insert fields like goals/exec_mode before updated/started (keep timestamps at end)
        let insert_pos = lines.iter().position(|l| {
            l.starts_with("updated:") || l.starts_with("started:")
        });
        if let Some(pos) = insert_pos {
            lines.insert(pos, format!("{}: {}", key, value));
        } else {
            lines.push(format!("{}: {}", key, value));
        }
    }

    // Ensure updated/started are always at the end
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

    // Ensure file ends with newline
    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    fs::write(path, output)
}

/// Read array field (e.g., `  - item` list under docs:)
pub fn get_list(path: &Path, key: &str) -> std::io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(parse_list(&content, key))
}

/// Parse array field
fn parse_list(content: &str, key: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut in_list = false;
    let prefix = format!("{}:", key);

    for line in content.lines() {
        if !in_list {
            let trimmed = line.trim();
            if trimmed.starts_with(&prefix) {
                let after_colon = trimmed[prefix.len()..].trim();
                if after_colon.is_empty() {
                    in_list = true;
                }
            }
        } else if line.starts_with("  - ") || line.starts_with("\t- ") {
            let item = line.trim().trim_start_matches("- ").trim().to_string();
            items.push(item);
        } else if line.trim().is_empty() {
            continue;
        } else {
            break;
        }
    }
    items
}

/// Set array field (delete field if array is empty)
pub fn set_list(path: &Path, key: &str, values: &[String]) -> std::io::Result<()> {
    let content = fs::read_to_string(path)?;
    let prefix = format!("{}:", key);
    let mut lines: Vec<String> = Vec::new();
    let mut skip_old_list = false;

    for line in content.lines() {
        if skip_old_list {
            if line.starts_with("  - ") || line.starts_with("\t- ") {
                continue;
            } else if line.trim().is_empty() {
                continue;
            } else {
                skip_old_list = false;
                lines.push(line.to_string());
            }
        } else {
            let trimmed = line.trim();
            if trimmed.starts_with(&prefix) && trimmed[prefix.len()..].trim().is_empty() {
                skip_old_list = true;
                continue;
            }
            lines.push(line.to_string());
        }
    }

    if !values.is_empty() {
        let insert_pos = lines.iter().position(|l| {
            l.starts_with("updated:") || l.starts_with("started:")
        });
        let mut block = vec![format!("{}:", key)];
        for v in values {
            block.push(format!("  - {}", v));
        }
        if let Some(pos) = insert_pos {
            for (i, line) in block.into_iter().enumerate() {
                lines.insert(pos + i, line);
            }
        } else {
            lines.extend(block);
        }
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    fs::write(path, output)
}

/// Update updated timestamp
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

    #[test]
    fn test_parse_list_basic() {
        let content = "name: test\ndocs:\n  - docs/structure.md\n  - docs/usage.md\nupdated: 2026-01-01\n";
        let items = parse_list(content, "docs");
        assert_eq!(items, vec!["docs/structure.md", "docs/usage.md"]);
    }

    #[test]
    fn test_parse_list_missing_key() {
        let content = "name: test\nphase: DEV\n";
        let items = parse_list(content, "docs");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_list_empty() {
        let content = "name: test\ndocs:\nupdated: 2026-01-01\n";
        let items = parse_list(content, "docs");
        assert!(items.is_empty());
    }

    #[test]
    fn test_set_list_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("STATUS.yaml");
        fs::write(&path, "name: test\nupdated: 2026-01-01\nstarted: 2026-01-01\n").unwrap();

        let values = vec!["docs/a.md".to_string(), "docs/b.md".to_string()];
        set_list(&path, "docs", &values).unwrap();

        let result = get_list(&path, "docs").unwrap();
        assert_eq!(result, values);

        // verify updated/started are still at the end
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.ends_with("updated: 2026-01-01\nstarted: 2026-01-01\n"));
    }

    #[test]
    fn test_set_list_empty_removes_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("STATUS.yaml");
        fs::write(&path, "name: test\ndocs:\n  - docs/a.md\nupdated: 2026-01-01\nstarted: 2026-01-01\n").unwrap();

        set_list(&path, "docs", &[]).unwrap();

        let result = get_list(&path, "docs").unwrap();
        assert!(result.is_empty());
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("docs:"));
    }
}
