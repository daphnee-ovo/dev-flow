// dow/src/commands/
// ├── version.rs  -- dow version（读写 VERSION 文件，委托 core::version）

use crate::cli::VersionArgs;
use crate::core::version;
use crate::error::DowError;
use crate::output;
use serde::Serialize;

#[derive(Serialize)]
struct VersionOutput {
    version: String,
    branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
}

pub fn run(args: VersionArgs, human: bool) -> Result<i32, DowError> {
    let branch = crate::core::doc_root::current_branch()
        .unwrap_or_else(|| "main".to_string());
    let current = version::read_current()?;

    if let Some(ref new_ver) = args.set {
        version::write_current(new_ver)?;
        let result = VersionOutput {
            version: new_ver.clone(),
            branch: branch.clone(),
            previous: Some(current),
            action: Some("set".to_string()),
        };
        if human {
            println!("[dev-flow] version({}): {} → {}", branch, result.previous.as_ref().unwrap(), result.version);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    if let Some(ref bump_type) = args.bump {
        let (prev, new_ver) = version::bump(bump_type)?;
        let result = VersionOutput {
            version: new_ver,
            branch: branch.clone(),
            previous: Some(prev),
            action: Some(format!("bump:{}", bump_type)),
        };
        if human {
            println!("[dev-flow] version({}): {} → {} ({})", branch, result.previous.as_ref().unwrap(), result.version, bump_type);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    // 只读
    let result = VersionOutput {
        version: current,
        branch,
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
