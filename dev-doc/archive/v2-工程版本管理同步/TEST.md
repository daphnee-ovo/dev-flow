# 测试报告

- 执行时间：2026-05-19 14:57
- 测试范围：v2 迭代 7 个 P0 issue 修复验证
- 总用例数：41
- 通过：40
- 失败：1

## 失败用例

| 模块 | 用例 | 错误信息 | 关联 issue |
|------|------|----------|-----------|
| save-changelog | TEST 3.4: 中文 commit message 不应导致 topic 为空 | topic 为空（tr -cd [:print:] 删除中文） | issue_test_2026-05-19_1 I1 |

## 通过模块

- validate.sh 内容结构校验（9/9）
- block-non-dev-edit.sh 阶段守卫（8/8）
- save-changelog.sh 安全性（4/5 - 1 fail）
- hooks.json 注册验证（2/2）
- init.md 规范对照指令（2/2）
- init.md warning 处理指令（4/4）
- test_validate.sh 覆盖验证（4/4）
- 边界情况测试（5/5）

## 附加发现

已有测试 test_save_changelog.sh 有 2 个失败（PASS:6 FAIL:2），原因：
1. 测试期望 "# Changelog"（符合 SPEC）但实现写 "# CHANGELOG" -- P2 不一致
2. 测试用中文 commit message 但 tr 删除中文导致 topic 丢失 -- 同 I1

## 各 issue 修复验证结论

| Issue | 修复内容 | 验证结果 |
|-------|----------|----------|
| 1 | validate.sh 内容结构校验 | PASS - nums/格式/字段/severity 全部正确检测 |
| 2 | init.md 规范对照指令 | PASS - 阶段 3 有规范对照要求且指向正确路径 |
| 3 | test_validate.sh 覆盖 | PASS - TEST 9-12 覆盖全部新增校验逻辑 |
| 4 | 阶段守卫 hook | PASS - 非 DEV 阶段阻断源码修改，白名单正确放行 |
| 5 | init.md warning 处理 | PASS - 4 种新 warning 类型均有处理指令 |
| 6 | save-changelog.sh 安全性 | PARTIAL - 不用 sed -i、不产生 binary、grep 有 -a；但 tr 删除中文是新问题 |
| 7 | hooks.json 注册 | PASS - block-non-dev-edit.sh 已在 PreToolUse Write|Edit 下注册 |
