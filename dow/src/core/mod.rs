// dow/src/core/
// ├── mod.rs            -- 公共库入口
// ├── yaml.rs           -- STATUS.yaml 轻量读写
// ├── doc_root.rs       -- doc_root 解析逻辑
// ├── archive_db.rs     -- SQLite 归档存储
// ├── version.rs        -- VERSION 文件多分支读写
// ├── doc_validator.rs  -- .dev-doc 文件合法性校验（从 md 规范提取规则）

/// dev-flow 文档根目录名
pub const DOC_DIR: &str = ".dev-doc";

/// 旧版文档目录名（用于迁移检测）
pub const DOC_DIR_LEGACY: &str = "dev-doc";

pub mod archive_db;
pub mod doc_root;
pub mod doc_validator;
pub mod version;
pub mod yaml;
