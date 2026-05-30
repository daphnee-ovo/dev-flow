// dow/src/commands/
// ├── mod.rs       -- 子命令入口
// ├── archive.rs   -- SQLite 归档查询
// ├── status.rs    -- 读写 STATUS.yaml
// ├── validate.rs  -- 校验 .dev-doc
// ├── fix.rs       -- 自动修复 .dev-doc 格式
// ├── scan.rs      -- 项目扫描
// ├── doc.rs       -- 文档模板生成

pub mod archive;
pub mod check;
pub mod devtest;
pub mod doc;
pub mod fix;
pub mod info;
pub mod init;
pub mod issue;
pub mod iterate;
pub mod scan;
pub mod self_check;
pub mod setup;
pub mod status;
pub mod test_runner;
pub mod update;
pub mod validate;
pub mod version;
