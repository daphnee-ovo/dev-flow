---
description: 执行交付检查 — 确认项目可以交付
allowed-tools: Bash, Read, Write, Edit
---

# DONE — 交付确认

## 前置检查（阻断）

逐条硬检查，任何一条不通过都**停止并报告**：

```bash
DOC_ROOT="dev-doc"  # 或多工程模式下的路径

# 1. TASK 全部完成
UNDONE=$(grep -c "^- \[ \]" "$DOC_ROOT/TASK.md" 2>/dev/null || echo 999)
[ "$UNDONE" -gt 0 ] && echo "BLOCKED: $UNDONE 个任务未完成" && exit 1

# 2. 无未关闭 issue
OPEN=$(find "$DOC_ROOT/issue" -name "*.md" ! -name "*.closed.md" 2>/dev/null | wc -l)
[ "$OPEN" -gt 0 ] && echo "BLOCKED: $OPEN 个未关闭 issue" && exit 1

# 3. TEST.md 存在
[ ! -f "$DOC_ROOT/TEST.md" ] && echo "BLOCKED: 未执行项目测试" && exit 1
```

## 执行方式

由**主 agent 直接执行**（不启动独立 subagent）。逐项检查交付清单。

## 交付清单

- [ ] TASK.md 所有任务已勾选 `[x]`
- [ ] `dev-doc/issue/` 中无未关闭 issue（所有 .md 都是 .closed.md）
- [ ] TEST.md 存在且有测试结果
- [ ] SPEC.md 与实际代码一致（抽查关键接口/数据模型）
- [ ] 代码可正常运行（执行启动命令，无报错）
- [ ] session 记录已保存

## SPEC 一致性抽查

读取 SPEC.md 中的：
- 接口列表 → 实际检查对应路由/函数是否存在
- 数据模型 → 检查对应 schema/struct 是否匹配
- 目录结构 → 对比实际目录

如果发现不一致 → 报告具体差异，不自动修复（让用户决定是改 SPEC 还是改代码）。

## 完成后

1. 更新 STATUS.md：当前阶段 → DONE，勾选所有阶段
2. 输出交付报告
