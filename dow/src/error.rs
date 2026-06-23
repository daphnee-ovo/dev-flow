// dow/src/
// ├── error.rs  -- Unified error type

use std::fmt;

#[derive(Debug)]
pub struct DowError {
    pub message: String,
    pub exit_code: i32,
}

impl DowError {
    pub fn new(message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }
}

impl fmt::Display for DowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DowError {}
