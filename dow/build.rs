use std::fs;

fn main() {
    let version_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../VERSION");
    let content = fs::read_to_string(version_path).unwrap_or_else(|_| "0.0.0".to_string());
    // VERSION 格式: (branch)X.Y.Z，提取版本号
    let version = if let Some(pos) = content.find(')') {
        content[pos + 1..].trim()
    } else {
        content.trim()
    };
    println!("cargo:rustc-env=DOW_VERSION={}", version);
    println!("cargo:rerun-if-changed=../VERSION");
}
