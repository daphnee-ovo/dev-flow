---
source: other
nums: 1
---

- [x] ISSUE-I002：validate_no_illegal_files 应跳过隐藏文件
  - severity: P1
  - location：dow/src/core/doc_validator.rs:938
  - description：validate_no_illegal_files 遍历 .dev-doc 目录时未跳过以 . 开头的隐藏文件（如 .DS_Store），macOS 自动生成这些文件无法避免，不应视为非法文件报错。同样 task/ 和 issue/ 子目录遍历也需跳过隐藏文件
  - fix：添加 should_ignore 判断，硬编码跳过 OS 生成文件（.DS_Store/Thumbs.db/desktop.ini），同时解析 .gitignore 中的模式也予以跳过

