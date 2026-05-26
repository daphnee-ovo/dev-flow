// dow/src/commands/
// ├── mod.rs       -- 子命令入口
// ├── status.rs    -- 读写 STATUS.yaml
// ├── validate.rs  -- 校验 dev-doc
// ├── scan.rs      -- 项目扫描
// ├── doc.rs       -- 文档模板生成

pub mod check;
pub mod devtest;
pub mod doc;
pub mod info;
pub mod init;
pub mod issue;
pub mod iterate;
pub mod scan;
pub mod status;
pub mod test_runner;
pub mod validate;
pub mod version;
