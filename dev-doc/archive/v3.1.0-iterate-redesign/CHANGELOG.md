# Changelog

## 2026-05-26
- feat: dow iterate 预览→确认流程，使用环境变量 token 机制
- feat: dow iterate --files 参数显式指定提交文件
- feat: dow iterate --type 必填参数，commit message 遵循 conventional commit
- feat: dow iterate 读取 CHANGELOG 作为 commit body
- fix: git_commit 改用 git add -u 替代 git add -A，避免误提交
