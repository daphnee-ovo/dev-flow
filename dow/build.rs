use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let version_path = Path::new(manifest_dir).parent().unwrap().join("VERSION");
    let content = fs::read_to_string(&version_path).unwrap_or_else(|_| "0.0.0".to_string());
    // VERSION 格式: (branch)X.Y.Z，提取版本号
    let version = if let Some(pos) = content.find(')') {
        content[pos + 1..].trim()
    } else {
        content.trim()
    };
    println!("cargo:rustc-env=DOW_VERSION={}", version);
    println!("cargo:rerun-if-changed={}", version_path.display());
}
