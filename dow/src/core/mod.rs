// dow/src/core/
// ├── mod.rs         -- 公共库入口
// ├── yaml.rs        -- STATUS.yaml 轻量读写
// ├── doc_root.rs    -- doc_root 解析逻辑
// ├── archive_db.rs  -- SQLite 归档存储

pub mod archive_db;
pub mod doc_root;
pub mod yaml;
