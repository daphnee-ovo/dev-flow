// dow/src/commands/
// ├── version.rs  -- dow version（读写 VERSION 文件）

use crate::cli::VersionArgs;
use crate::error::DowError;
use crate::output;
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
struct VersionOutput {
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
}

pub fn run(args: VersionArgs, human: bool) -> Result<i32, DowError> {
    let current = read_version()?;

    if let Some(ref new_ver) = args.set {
        validate_semver(new_ver)?;
        write_version(new_ver)?;
        let result = VersionOutput {
            version: new_ver.clone(),
            previous: Some(current),
            action: Some("set".to_string()),
        };
        if human {
            println!("[dev-flow] version: {} → {}", result.previous.as_ref().unwrap(), result.version);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    if let Some(ref bump_type) = args.bump {
        let new_ver = bump_version(&current, bump_type)?;
        write_version(&new_ver)?;
        let result = VersionOutput {
            version: new_ver,
            previous: Some(current),
            action: Some(format!("bump:{}", bump_type)),
        };
        if human {
            println!("[dev-flow] version: {} → {} ({})", result.previous.as_ref().unwrap(), result.version, bump_type);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    // 只读
    let result = VersionOutput {
        version: current,
        previous: None,
        action: None,
    };
    if human {
        println!("{}", result.version);
    } else {
        output::print_json(&result);
    }
    Ok(0)
}

fn read_version() -> Result<String, DowError> {
    fs::read_to_string("VERSION")
        .map(|s| s.trim().to_string())
        .map_err(|_| DowError::new("VERSION 文件不存在或不可读", 1))
}

fn write_version(version: &str) -> Result<(), DowError> {
    fs::write("VERSION", format!("{}\n", version))
        .map_err(|e| DowError::new(e.to_string(), 1))
}

fn validate_semver(version: &str) -> Result<(), DowError> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err(DowError::new(format!("版本格式非法（需要 X.Y.Z）：{}", version), 1));
    }
    for part in &parts {
        if part.parse::<u32>().is_err() {
            return Err(DowError::new(format!("版本格式非法（非数字）：{}", version), 1));
        }
    }
    Ok(())
}

fn bump_version(version: &str, bump_type: &str) -> Result<String, DowError> {
    let parts: Vec<u32> = version
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .collect();

    if parts.len() != 3 {
        return Err(DowError::new(format!("版本格式非法：{}", version), 1));
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    match bump_type {
        "major" => Ok(format!("{}.0.0", major + 1)),
        "minor" => Ok(format!("{}.{}.0", major, minor + 1)),
        "patch" => Ok(format!("{}.{}.{}", major, minor, patch + 1)),
        _ => Err(DowError::new(format!("未知 bump 类型：{}", bump_type), 1)),
    }
}
