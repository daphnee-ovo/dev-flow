// dow/src/hooks/
// ├── mod.rs           -- hook 子命令入口
// ├── context.rs       -- inject-context
// ├── guard.rs         -- block-system-tmp + block-non-dev-edit
// ├── post_write.rs    -- 写后联动
// ├── save_changelog.rs -- 保存 CHANGELOG

pub mod context;
pub mod guard;
pub mod post_write;
pub mod save_changelog;
