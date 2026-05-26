// dow/src/
// ├── output.rs  -- JSON / human 输出切换

use serde::Serialize;

/// JSON 模式输出 serde 结构体，human 模式输出格式化文本
pub fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

pub fn print_line(text: &str) {
    println!("{}", text);
}
