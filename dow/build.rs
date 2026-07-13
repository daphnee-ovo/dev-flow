use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().unwrap();
    let version_path = project_root.join("VERSION");
    let content = fs::read_to_string(&version_path).unwrap_or_else(|_| "0.0.0".to_string());

    // Detect current git branch
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());
    let branch = branch.trim();

    // VERSION 格式: multi-line "(branch)X.Y.Z", find matching branch line
    let version = content
        .lines()
        .find_map(|line| {
            let pattern = format!("({})", branch);
            if line.starts_with(&pattern) {
                Some(line[pattern.len()..].trim())
            } else {
                None
            }
        })
        .or_else(|| {
            // Fallback: first line with (branch)version format
            content
                .lines()
                .next()
                .and_then(|line| line.find(')').map(|pos| line[pos + 1..].trim()))
        })
        .unwrap_or("0.0.0");

    println!("cargo:rustc-env=DOW_VERSION={}", version);
    println!("cargo:rerun-if-changed={}", version_path.display());
}
