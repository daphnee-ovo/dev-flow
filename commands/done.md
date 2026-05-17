---
description: 执行交付检查 — 确认项目可以交付
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# DONE — 交付确认

## 模式检测

```bash
if find dev-doc -maxdepth 2 -name "STATUS.yaml" -path "*/*/STATUS.yaml" 2>/dev/null | grep -q .; then
  BRANCH=$(git branch --show-current 2>/dev/null)
  DOC_ROOT="dev-doc/$BRANCH"
else
  DOC_ROOT="dev-doc"
fi

MODE=$(grep "^mode:" "$DOC_ROOT/STATUS.yaml" | sed 's/^mode: *//')
```

## 按模式分级检查

根据 STATUS.yaml 的 mode 确定检查项：

```bash
BLOCKED=""

case "$MODE" in
  full)
    [ ! -f "$DOC_ROOT/PRD.md" ] && BLOCKED="缺少 PRD.md"
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    ;;
  quick)
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    ;;
  fast)
    ;;
  mvp)
    [ ! -f "$DOC_ROOT/SPEC.md" ] && BLOCKED="缺少 SPEC.md"
    ;;
esac

# 通用检查（除 mvp 外）
if [ "$MODE" != "mvp" ]; then
  # task 全完成
  UNDONE=0
  for f in "$DOC_ROOT/task/task_"*.md; do
    [ -f "$f" ] || continue
    CNT=$(grep -c "^- \[ \]" "$f" 2>/dev/null) || true; UNDONE=$((UNDONE + ${CNT:-0}))
  done
  [ "$UNDONE" -gt 0 ] && BLOCKED="$UNDONE 个任务未完成"

  # TEST.md 存在
  [ ! -f "$DOC_ROOT/TEST.md" ] && BLOCKED="未执行项目测试"

  # 无 P0 issue
  P0_OPEN=0
  for f in "$DOC_ROOT/issue/issue_"*.md; do
    [ -f "$f" ] || continue
    if grep -q "severity: P0" "$f" && grep -q "^- \[ \]" "$f"; then
      P0_OPEN=$((P0_OPEN + 1))
    fi
  done
  [ "$P0_OPEN" -gt 0 ] && BLOCKED="$P0_OPEN 个 P0 issue 未关闭"
fi
```

mvp 模式特殊处理：只要求 SPEC 存在 + 代码可运行（询问用户确认）。

## 执行方式

由**主 agent 直接执行**（不启动独立 subagent）。逐项检查交付清单。

## 交付清单

| 模式 | 检查项 |
|------|--------|
| full | PRD + SPEC + task 全完成 + TEST 全过 + 无 P0 issue |
| quick | SPEC + task 全完成 + TEST 全过 + 无 P0 issue |
| fast | task 全完成 + TEST 全过 + 无 P0 issue |
| mvp | SPEC 存在 + 代码可运行（用户确认） |

所有模式通用：
- [ ] SPEC.md 与实际代码一致（抽查关键接口/数据模型）
- [ ] 代码可正常运行（执行启动命令，无报错）

## SPEC 一致性抽查

读取 SPEC.md 中的：
- 接口列表 → 实际检查对应路由/函数是否存在
- 数据模型 → 检查对应 schema/struct 是否匹配
- 目录结构 → 对比实际目录

如果发现不一致 → 报告具体差异，不自动修复（让用户决定是改 SPEC 还是改代码）。

## 完成后

1. 更新 STATUS.yaml：当前阶段 → DONE
2. 输出交付报告
