// dow/src/
// ├── output.rs  -- JSON / human output toggle

use serde::Serialize;

/// JSON mode outputs serde struct, human mode outputs formatted text
pub fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}
