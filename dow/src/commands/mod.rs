// dow/src/commands/
// ├── mod.rs       -- Subcommand entry point
// ├── archive.rs   -- SQLite archive queries
// ├── status.rs    -- Read/write STATUS.yaml
// ├── validate.rs  -- Validate .dev-doc structure
// ├── fix.rs       -- Auto-fix .dev-doc format issues
// ├── scan.rs      -- Project scanning
// ├── doc.rs       -- Document template generation

pub mod archive;
pub mod check;
pub mod claim;
pub mod devtest;
pub mod doc;
pub mod fix;
pub mod info;
pub mod init;
pub mod issue;
pub mod iterate;
pub mod revoke;
pub mod scan;
pub mod self_check;
pub mod setup;
pub mod status;
pub mod test_runner;
pub mod update;
pub mod validate;
pub mod version;
