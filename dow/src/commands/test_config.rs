use crate::error::DowError;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub(crate) struct TestCiConfig {
    pub(crate) devtest: Vec<String>,
    pub(crate) test: Vec<String>,
}

#[derive(Clone, Copy)]
enum Section {
    Devtest,
    Test,
}

pub(crate) fn load(project_root: &Path) -> Result<Option<TestCiConfig>, DowError> {
    let path = project_root.join(".dev-doc/test.ci");
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| DowError::new(format!("cannot read {}: {}", path.display(), e), 2))?;
    let mut config = TestCiConfig::default();
    let mut section: Option<Section> = None;

    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let is_top_level = line
            .chars()
            .next()
            .map(|character| !character.is_whitespace())
            .unwrap_or(false);
        if is_top_level && trimmed.ends_with(':') {
            section = match trimmed.trim_end_matches(':') {
                "devtest" => Some(Section::Devtest),
                "test" => Some(Section::Test),
                other => {
                    return Err(DowError::new(
                        format!(
                            "invalid test.ci section '{}' at line {}",
                            other,
                            line_number + 1
                        ),
                        2,
                    ));
                }
            };
            continue;
        }

        let Some(command) = trimmed.strip_prefix("run:").map(str::trim) else {
            return Err(DowError::new(
                format!("invalid test.ci entry at line {}", line_number + 1),
                2,
            ));
        };
        if command.is_empty() {
            return Err(DowError::new(
                format!("empty test.ci run at line {}", line_number + 1),
                2,
            ));
        }

        match section {
            Some(Section::Devtest) => config.devtest.push(command.to_string()),
            Some(Section::Test) => config.test.push(command.to_string()),
            None => {
                return Err(DowError::new(
                    format!(
                        "test.ci run appears before a section at line {}",
                        line_number + 1
                    ),
                    2,
                ));
            }
        }
    }

    Ok(Some(config))
}
