// dow/src/hooks/
// ├── mod.rs           -- hook subcommand entry
// ├── context.rs       -- inject-context
// ├── guard.rs         -- block-system-tmp + block-non-dev-edit
// ├── post_write.rs    -- post-write hooks
// ├── session_stop.rs  -- unified session end (revoke claims + save changelog)

pub mod context;
pub mod guard;
pub mod post_bash;
pub mod post_write;
pub mod session_stop;
