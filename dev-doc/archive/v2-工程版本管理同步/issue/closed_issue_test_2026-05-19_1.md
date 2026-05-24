---
source: test
nums: 2
---

- [x] I1：save-changelog.sh 中 tr -cd '[:print:]' 删除中文字符导致 topic 为空
  - severity: P1
  - location：scripts/hooks/save-changelog.sh:34
  - description：第 34 行 `TOPIC=$(echo "$TOPIC" | tr -cd '[:print:]' | sed 's/[&/\\]/\\&/g')` 中的 `tr -cd '[:print:]'` 在 C.UTF-8 locale 下会删除所有非 ASCII 字符（包括中文），导致中文 commit message 作为 topic 时变为空字符串。CHANGELOG 记录变为 `- HH:MM ` 无内容。
  - reproduce：1) git commit --allow-empty -m "修复登录验证逻辑" 2) 运行 save-changelog.sh 3) 查看 CHANGELOG.md 最后一行，topic 部分为空
  - fix：改用 sed 's/[[:cntrl:]]//g' 只清除控制字符，保留中文等多字节字符

- [x] I2：save-changelog.sh 和 validate.sh 的 CHANGELOG 头部写 "# CHANGELOG" 与 SPEC 定义的 "# Changelog" 不一致
  - severity: P2
  - location：scripts/hooks/save-changelog.sh:30
  - description：SPEC 3.3 节明确定义 CHANGELOG 格式头部为 `# Changelog`，但 save-changelog.sh 第 30 行写 `printf "# CHANGELOG\n\n"` 且 validate.sh 第 146 行写 `echo "# CHANGELOG"`。二者自身一致但与 SPEC 模板不一致。不影响功能（grep 匹配用的是 `^## ` 日期段），但导致现有 test_save_changelog.sh 中 2 个测试失败。
  - reproduce：1) 删除 CHANGELOG.md 2) 运行 save-changelog.sh 或 validate.sh 3) 查看输出头部为 "# CHANGELOG" 而非 SPEC 定义的 "# Changelog"
  - fix：统一改为 "# Changelog"，与 SPEC 3.3 定义一致
