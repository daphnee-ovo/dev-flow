// FrameworkTree
// input_validation.rs
// ├── struct ValidationErrors
// ├── impl ValidationErrors
// ├── push()
// ├── len()
// ├── finish()
// ├── field_path()
// ├── object()
// ├── unknown_fields()
// ├── required_string()
// ├── optional_string()
// ├── required_string_array()
// ├── optional_string_array()
// ├── string_array()
// ├── required_bool()
// ├── optional_bool()
// ├── invalid_json_error()
// ├── display_path()
// └── json_type()

use crate::error::DowError;
use serde_json::{Map, Value};

/// Ordered validation diagnostics shared by CLI input adapters.
#[derive(Default)]
pub(crate) struct ValidationErrors {
    messages: Vec<String>,
}

impl ValidationErrors {
    pub(crate) fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    pub(crate) fn len(&self) -> usize {
        self.messages.len()
    }

    pub(crate) fn finish(self, context: &str, hint: &str) -> Result<(), DowError> {
        if self.messages.is_empty() {
            return Ok(());
        }

        let count = self.messages.len();
        let noun = if count == 1 { "error" } else { "errors" };
        let mut message = format!("{} input validation failed ({} {}):", context, count, noun);
        for diagnostic in self.messages {
            message.push_str("\n- ");
            message.push_str(&diagnostic);
        }
        if !hint.is_empty() {
            message.push_str("\nHint: ");
            message.push_str(hint);
        }

        Err(DowError::new(message, 2))
    }
}

pub(crate) fn field_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_string()
    } else {
        format!("{}.{}", prefix, field)
    }
}

pub(crate) fn object<'a>(
    value: &'a Value,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<&'a Map<String, Value>> {
    match value.as_object() {
        Some(object) => Some(object),
        None => {
            errors.push(format!(
                "{}: expected a JSON object, got {}",
                display_path(path),
                json_type(value)
            ));
            None
        }
    }
}

pub(crate) fn unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
    errors: &mut ValidationErrors,
) {
    let mut unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !allowed.contains(key))
        .collect();
    unknown.sort_unstable();
    if !unknown.is_empty() {
        errors.push(format!(
            "{}: unknown field{} {}; allowed: {}",
            display_path(path),
            if unknown.len() == 1 { "" } else { "s" },
            unknown.join(", "),
            allowed.join(", ")
        ));
    }
}

pub(crate) fn required_string(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<String> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => {
            errors.push(format!(
                "{}: missing (expected a string)",
                display_path(&field_path)
            ));
            None
        }
        Some(value) => value.as_str().map(ToOwned::to_owned).or_else(|| {
            errors.push(format!(
                "{}: expected a string, got {}",
                display_path(&field_path),
                json_type(value)
            ));
            None
        }),
    }
}

pub(crate) fn optional_string(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<String> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => None,
        Some(value) => value.as_str().map(ToOwned::to_owned).or_else(|| {
            errors.push(format!(
                "{}: expected a string, got {}",
                display_path(&field_path),
                json_type(value)
            ));
            None
        }),
    }
}

pub(crate) fn required_string_array(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<Vec<String>> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => {
            errors.push(format!(
                "{}: missing (expected an array of strings)",
                display_path(&field_path)
            ));
            None
        }
        Some(value) => string_array(value, &field_path, errors),
    }
}

pub(crate) fn optional_string_array(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<Vec<String>> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => None,
        Some(value) => string_array(value, &field_path, errors),
    }
}

fn string_array(value: &Value, path: &str, errors: &mut ValidationErrors) -> Option<Vec<String>> {
    let Some(values) = value.as_array() else {
        errors.push(format!(
            "{}: expected an array of strings, got {}",
            display_path(path),
            json_type(value)
        ));
        return None;
    };

    let before = errors.len();
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if let Some(item) = value.as_str() {
            result.push(item.to_string());
        } else {
            errors.push(format!(
                "{}[{}]: expected a string, got {}",
                display_path(path),
                index,
                json_type(value)
            ));
        }
    }

    if errors.len() == before {
        Some(result)
    } else {
        None
    }
}

pub(crate) fn required_bool(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<bool> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => {
            errors.push(format!(
                "{}: missing (expected a boolean)",
                display_path(&field_path)
            ));
            None
        }
        Some(value) => value.as_bool().or_else(|| {
            errors.push(format!(
                "{}: expected a boolean, got {}",
                display_path(&field_path),
                json_type(value)
            ));
            None
        }),
    }
}

pub(crate) fn optional_bool(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    errors: &mut ValidationErrors,
) -> Option<bool> {
    let field_path = field_path(path, field);
    match object.get(field) {
        None => None,
        Some(value) => value.as_bool().or_else(|| {
            errors.push(format!(
                "{}: expected a boolean, got {}",
                display_path(&field_path),
                json_type(value)
            ));
            None
        }),
    }
}

pub(crate) fn invalid_json_error(context: &str, error: &serde_json::Error, hint: &str) -> DowError {
    DowError::new(
        format!(
            "{}: invalid JSON at line {}, column {}: {}\nHint: {}",
            context,
            error.line(),
            error.column(),
            error,
            hint
        ),
        2,
    )
}

fn display_path(path: &str) -> String {
    if path.is_empty() {
        "input".to_string()
    } else {
        path.to_string()
    }
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
