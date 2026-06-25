# Changelog

## 2026-06-22
- 10:57 ci: update dow platform binaries

## 2026-06-23
- 12:21 fix: ISSUE-I002：validate_no_illegal_files 应跳过隐藏文件
- 12:28 fix: validate_no_illegal_files 跳过 OS 生成文件和 .gitignore 匹配文件
- 12:30 fix: ISSUE-I003：dow fix 不处理 issue closed_ 重命名
- 12:46 fix: dow fix 支持 closed_/done_ 重命名和全局序号重编号
- 12:49 fix: install.sh 下载增加进度显示、重试和超时
- 12:55 fix: ISSUE-I009：全项目默认语言改为英文
- 14:55 feat: translate entire project to English

## 2026-06-24
- 11:07 docs: fix doc-code drift in hooks and structure docs
- 11:15 workflow: update dev-flow state
- 14:41 docs: 禁止在 dow/ 目录下创建 .dev-doc
- 17:55 refactor: 将 agent_registry.rs 内嵌 prompt 提取到 dow/references/inject_prompt/ 外部文件

## 2026-06-25
- 12:21 refactor: cli.rs 重写 Commands enum
- 12:59 feat: 实现 dow task 子命令（create/list/show/done/reopen/schema）
- 13:03 docs: 更新注入 prompt 和 --help 匹配新命令体系
- 13:05 feat: dow task done 支持多参数批量标记完成
- 13:07 feat: dow issue close 支持多参数批量关闭
- 14:07 fix: 修复测试适配新命令 + iterate confirm 去除 env var 依赖
- 14:19 fix: task batch file 序号冲突覆盖 bug
- 14:31 fix: lint --fix 后重新 validate 确保 errors 反映修复后状态
