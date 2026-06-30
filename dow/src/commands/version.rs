// dow/src/commands/
// ├── version.rs  -- dow version (read/write VERSION file, delegates to core::version)

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
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

pub fn run(args: VersionArgs, human: bool) -> Result<i32, DowError> {
    let detached = version::is_detached();
    let branch = version::resolve_branch();
    let warning = if detached {
        Some("cannot detect current branch (detached HEAD), falling back to 'main'".to_string())
    } else {
        None
    };

    // --set / --bump require a real branch — do not fallback
    if args.set.is_some() || args.bump.is_some() {
        if detached {
            return Err(DowError::new(
                "Cannot write version in detached HEAD state — checkout a branch first",
                1,
            ));
        }
    }

    if let Some(ref new_ver) = args.set {
        let previous = version::read_current().ok();
        version::write_current(new_ver)?;
        let result = VersionOutput {
            version: new_ver.clone(),
            branch: branch.clone(),
            previous,
            action: Some("set".to_string()),
            warning: None,
        };
        if human {
            let prev_str = result.previous.as_deref().unwrap_or("(damaged)");
            println!("[dev-flow] version({}): {} → {}", branch, prev_str, result.version);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    let current = version::read_current()?;

    if let Some(ref bump_type) = args.bump {
        let (prev, new_ver) = version::bump(bump_type)?;
        let result = VersionOutput {
            version: new_ver,
            branch: branch.clone(),
            previous: Some(prev),
            action: Some(format!("bump:{}", bump_type)),
            warning: None,
        };
        if human {
            println!("[dev-flow] version({}): {} → {} ({})", branch, result.previous.as_ref().unwrap(), result.version, bump_type);
        } else {
            output::print_json(&result);
        }
        return Ok(0);
    }

    // Read-only
    let result = VersionOutput {
        version: current,
        branch,
        previous: None,
        action: None,
        warning: warning.clone(),
    };
    if human {
        if let Some(ref w) = warning {
            println!("[dow] WARNING: {}", w);
        }
        println!("{}", result.version);
    } else {
        output::print_json(&result);
    }
    if detached { Ok(2) } else { Ok(0) }
}
