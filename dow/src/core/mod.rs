// dow/src/core/
// ├── mod.rs            -- Common library entry
// ├── yaml.rs           -- STATUS.yaml lightweight read/write
// ├── doc_root.rs       -- doc_root resolution logic
// ├── archive_db.rs     -- SQLite archive storage
// ├── version.rs        -- VERSION file multi-branch read/write
// ├── doc_validator.rs  -- .dev-doc file validity validation (extract rules from md specs)
// ├── config.rs         -- ~/.config/dow/config.toml read/write
// ├── platform.rs       -- Platform detection, XDG path conventions
// ├── process.rs        -- Cross-platform process tree traversal for agent identity
// ├── github.rs         -- GitHub Release API interaction
// ├── agent_registry.rs -- Agent plugin directory discovery and file deployment

/// dev-flow document root directory name
pub const DOC_DIR: &str = ".dev-doc";

/// Legacy document directory name (for migration detection)
pub const DOC_DIR_LEGACY: &str = "dev-doc";

pub mod agent_registry;
pub mod archive_db;
pub mod claim;
pub mod config;
pub mod doc_root;
pub mod doc_validator;
pub mod github;
pub mod item_id;
pub mod platform;
pub mod process;
pub mod renumber;
pub mod task_store;
pub mod version;
pub mod yaml;
