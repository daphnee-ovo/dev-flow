use crate::core::item_id::{self, ItemKind};
use crate::error::DowError;
use crate::output;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashSet;

struct OrderedShowOutput(Vec<(String, Value)>);

impl Serialize for OrderedShowOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

const TASK_DEFAULT_FIELDS: &[&str] = &[
    "id",
    "title",
    "type",
    "priority",
    "status",
    "refs",
    "complexity",
    "depends_on",
    "parallel",
    "done_when",
];
const ISSUE_DEFAULT_FIELDS: &[&str] = &[
    "id",
    "title",
    "severity",
    "description",
    "reproduce",
    "status",
];
const TASK_FIELDS: &[&str] = &[
    "id",
    "title",
    "type",
    "priority",
    "status",
    "refs",
    "complexity",
    "depends_on",
    "parallel",
    "done_when",
    "files",
];
const ISSUE_FIELDS: &[&str] = &[
    "id",
    "title",
    "severity",
    "location",
    "description",
    "reproduce",
    "fix",
    "files",
    "status",
];

/// Shared implementation used by `dow show` and the legacy task/issue aliases.
pub fn run(
    ids: Vec<String>,
    expected_kind: Option<ItemKind>,
    human: bool,
    verbose: bool,
    fields: Option<Vec<String>>,
) -> Result<i32, DowError> {
    if ids.is_empty() {
        return Err(DowError::new("dow show requires at least one ID", 2));
    }

    let fields = parse_fields(fields)?;
    let parsed_ids = ids
        .iter()
        .map(|raw_id| {
            let parsed = item_id::parse(raw_id)
                .ok_or_else(|| DowError::new(format!("invalid item ID: {}", raw_id), 2))?;
            if let Some(expected) = expected_kind {
                if parsed.kind != expected {
                    return Err(DowError::new(
                        format!("{} is not a {} ID", raw_id, kind_name(expected)),
                        2,
                    ));
                }
            }
            Ok(parsed)
        })
        .collect::<Result<Vec<_>, DowError>>()?;

    if let Some(fields) = &fields {
        for field in fields {
            if !parsed_ids.iter().any(|id| field_available(id.kind, field)) {
                return Err(DowError::new(
                    format!("field {} is not available for the supplied IDs", field),
                    2,
                ));
            }
        }
    }

    let mut results = Vec::with_capacity(ids.len());
    let kinds: Vec<ItemKind> = parsed_ids.iter().map(|id| id.kind).collect();

    for parsed in parsed_ids {
        let id = parsed.full();
        let include_files = verbose || fields.as_ref().is_some_and(|set| set.contains("files"));
        let value = match parsed.kind {
            ItemKind::Task => crate::commands::task::show_value(&id, include_files)?,
            ItemKind::Issue => crate::commands::issue::show_value(&id, include_files)?,
        };
        results.push(select_fields(value, parsed.kind, fields.as_ref(), verbose)?);
    }

    if human {
        for (index, (kind, value)) in kinds.iter().zip(&results).enumerate() {
            if index > 0 {
                println!();
            }
            print_human(*kind, value);
        }
    } else if results.len() == 1 {
        output::print_json(&ordered_show_output(&results[0])?);
    } else {
        let ordered = results
            .iter()
            .map(ordered_show_output)
            .collect::<Result<Vec<_>, _>>()?;
        output::print_json(&ordered);
    }

    Ok(0)
}

fn ordered_show_output(value: &Value) -> Result<OrderedShowOutput, DowError> {
    let object = value
        .as_object()
        .ok_or_else(|| DowError::new("show result must be a JSON object", 1))?;
    let mut fields = Vec::with_capacity(object.len());
    for field in ["id", "title"] {
        let value = object
            .get(field)
            .ok_or_else(|| DowError::new(format!("show result is missing {}", field), 1))?;
        fields.push((field.to_string(), value.clone()));
    }
    fields.extend(
        object
            .iter()
            .filter(|(field, _)| field.as_str() != "id" && field.as_str() != "title")
            .map(|(field, value)| (field.clone(), value.clone())),
    );
    Ok(OrderedShowOutput(fields))
}

fn field_available(kind: ItemKind, field: &str) -> bool {
    match kind {
        ItemKind::Task => TASK_FIELDS.contains(&field),
        ItemKind::Issue => ISSUE_FIELDS.contains(&field),
    }
}

fn parse_fields(fields: Option<Vec<String>>) -> Result<Option<HashSet<String>>, DowError> {
    let Some(fields) = fields else {
        return Ok(None);
    };

    let mut selected = HashSet::new();
    for field in fields {
        let field = field.trim().to_ascii_lowercase();
        if field.is_empty() {
            return Err(DowError::new("--fields must contain field names", 2));
        }
        if !TASK_FIELDS.contains(&field.as_str()) && !ISSUE_FIELDS.contains(&field.as_str()) {
            return Err(DowError::new(format!("unknown show field: {}", field), 2));
        }
        selected.insert(field);
    }
    Ok(Some(selected))
}

fn select_fields(
    mut value: Value,
    kind: ItemKind,
    requested: Option<&HashSet<String>>,
    verbose: bool,
) -> Result<Value, DowError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| DowError::new("show result must be a JSON object", 1))?;

    let mut selected: HashSet<&str> = match requested {
        Some(fields) => fields.iter().map(String::as_str).collect(),
        None => match kind {
            ItemKind::Task => TASK_DEFAULT_FIELDS.iter().copied().collect(),
            ItemKind::Issue => ISSUE_DEFAULT_FIELDS.iter().copied().collect(),
        },
    };
    selected.insert("id");
    selected.insert("title");

    if verbose {
        match kind {
            ItemKind::Task => {
                selected.insert("files");
            }
            ItemKind::Issue => {
                selected.insert("location");
                selected.insert("files");
            }
        }
    }

    object.retain(|field, _| selected.contains(field.as_str()));
    Ok(value)
}

fn print_human(kind: ItemKind, value: &Value) {
    let id = string_field(value, "id");
    let title = string_field(value, "title");

    match kind {
        ItemKind::Task => {
            println!("id: {}", id);
            println!("title: {}", title);
            for field in [
                "type",
                "priority",
                "status",
                "refs",
                "complexity",
                "depends_on",
                "parallel",
                "done_when",
                "files",
            ] {
                if let Some(field_value) = value.get(field) {
                    print_task_field(field, field_value);
                }
            }
        }
        ItemKind::Issue => {
            println!("[dev-flow] Issue: {}", id);
            println!("━━━━━━━━━━━━━━━━━━━━━━");
            println!("  Title:       {}", title);
            for (field, label) in [
                ("severity", "Severity"),
                ("description", "Description"),
                ("reproduce", "Reproduce"),
                ("status", "Status"),
                ("location", "Location"),
                ("fix", "Fix"),
            ] {
                if let Some(field_value) = value.get(field) {
                    println!(
                        "  {:<13} {}",
                        format!("{}:", label),
                        display_value(field_value)
                    );
                }
            }
            if let Some(files) = value.get("files") {
                if let Some(files) = files.as_object() {
                    for (field, label) in [("modify", "Files modify"), ("create", "Files create")] {
                        if let Some(paths) = files.get(field).and_then(Value::as_array) {
                            if !paths.is_empty() {
                                let paths = paths
                                    .iter()
                                    .map(display_value)
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                println!("  {}: {}", label, paths);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn print_task_field(field: &str, value: &Value) {
    match field {
        "done_when" => {
            println!("  done_when:");
            let items = value.as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                println!("    []");
            } else {
                for item in items {
                    println!("    - {}", display_value(&item));
                }
            }
        }
        "files" => {
            println!("  files:");
            for field in ["create", "modify", "test"] {
                let items = value
                    .get(field)
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                println!(
                    "    {}: {:?}",
                    field,
                    items.iter().map(display_value).collect::<Vec<_>>()
                );
            }
        }
        _ => println!("  {}: {}", field, display_value(value)),
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn display_value(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn kind_name(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Task => "task",
        ItemKind::Issue => "issue",
    }
}
