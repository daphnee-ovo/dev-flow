// dow/src/hooks/
// ├── mod.rs           -- hook subcommand entry
// ├── context.rs       -- inject-context
// ├── guard.rs         -- block-system-tmp + block-non-dev-edit
// ├── post_write.rs    -- post-write hooks
// ├── save_changelog.rs -- save CHANGELOG

pub mod context;
pub mod guard;
pub mod post_bash;
pub mod post_write;
pub mod save_changelog;
