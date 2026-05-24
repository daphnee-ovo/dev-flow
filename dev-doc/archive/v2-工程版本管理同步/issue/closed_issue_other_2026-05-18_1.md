---
source: other
nums: 7
---

- [x] I1：validate.sh 缺少 issue 内容结构校验
  - severity: P0
  - location：scripts/init/validate.sh:62
  - description：校验脚本只检查命名和 frontmatter，不校验条目格式（I<N>：标题）、必需子字段（severity/location/description）、nums 一致性、severity 合法值
  - fix：确认已有校验逻辑完整覆盖（nums_mismatch/bad_item_format/missing_required_fields/invalid_severity）

- [x] I2：agent 指令未要求主动对照规范文档
  - severity: P0
  - location：commands/init.md
  - description：init 流程中处理校验结果时，agent 仅依赖脚本输出，未被指示主动读取 references/dev-doc/ 规范文档做交叉验证。脚本覆盖有限时问题会被遗漏
  - fix：在 init.md 阶段 3 添加规范对照要求，agent 处理 warnings 时必须读取对应 references/dev-doc/ 规范文档

- [x] I3：测试必须使用项目测试框架，禁止越界到 /tmp
  - severity: P0
  - location：tests/test_validate.sh
  - description：已有 tests/test_validate.sh 但 agent 未使用，而是在 /tmp 下手动创建文件测试。新增校验逻辑后也未更新测试文件
  - fix：新增 TEST 9-12 覆盖内容结构校验，测试使用项目 tmp/ 目录，25 个断言全部通过

- [x] I4：DONE 阶段不应允许直接修改代码
  - severity: P0
  - location：scripts/hooks/post-tool-use.sh
  - description：agent 在项目 DONE 状态下直接修改了 validate.sh，没有走 /iterate → task → dev 流程。hook 或 agent 指令需要阻止这种行为
  - fix：新建 PreToolUse hook block-non-dev-edit.sh，非 DEV 阶段阻止修改源码（白名单放行 dev-doc/tests/tmp 等）

- [x] I5：改完脚本未更新 commands/init.md 中 warning 处理指令
  - severity: P0
  - location：commands/init.md
  - description：新增了 4 种 warning 类型（issue_nums_mismatch/issue_bad_item_format/issue_missing_required_fields/issue_invalid_severity），但 init 命令中 agent 处理 warnings 的指令未配套更新
  - fix：在 init.md 阶段 3 补充 4 种新 warning 的处理方式（修正 nums/格式/字段/severity）

- [x] I6：save-changelog.sh 用 sed -i 注入导致 CHANGELOG 变 binary
  - severity: P0
  - location：scripts/hooks/save-changelog.sh:36
  - description：TOPIC 含中文或特殊字符时，sed -i 插入会引入异常字节，导致后续 grep 报 "binary file matches"。另外 header 创建用 "# Changelog" 但匹配用 "# CHANGELOG"，大小写不一致导致 sed 匹配失败。已临时修复（printf 替代 sed），但在 SPEC 阶段违规修改
  - fix：save-changelog.sh 改用 printf >> 追加替代 sed -i 插入，TOPIC 做特殊字符清理，inject-context.sh grep 加 -a flag

- [x] I7：agent 在 SPEC 阶段直接修改代码
  - severity: P0
  - location：scripts/hooks/inject-context.sh:151, scripts/hooks/save-changelog.sh
  - description：与 I4 同类问题。当前阶段是 SPEC，agent 仍然直接修改了 inject-context.sh 和 save-changelog.sh。说明 I4 的根因（缺少阶段守卫机制）尚未解决时，agent 会反复违规
  - fix：同 I4，block-non-dev-edit.sh 现在会阻止非 DEV 阶段的源码修改
