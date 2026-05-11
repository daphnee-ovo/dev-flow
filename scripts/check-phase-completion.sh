#!/bin/bash
# Hook: 阶段文档写完后，检查是否满足完成标准
# 触发时机：PostToolUse(Write|Edit) 当目标为 PRD.md/SPEC.md/TASK.md/TEST.md
# 支持单工程和多工程模式

if [ ! -d "dev-doc" ]; then
  exit 0
fi

CHANGED_FILE="$1"

if [[ ! "$CHANGED_FILE" == dev-doc/* ]]; then
  exit 0
fi

REL_PATH="${CHANGED_FILE#dev-doc/}"
FIRST_SEGMENT="${REL_PATH%%/*}"
if [ -f "dev-doc/$FIRST_SEGMENT/STATUS.md" ]; then
  DOC_ROOT="dev-doc/$FIRST_SEGMENT"
  TARGET_FILE="${REL_PATH#$FIRST_SEGMENT/}"
else
  DOC_ROOT="dev-doc"
  TARGET_FILE="$REL_PATH"
fi

ISSUES=""

case "$TARGET_FILE" in
  PRD.md)
    FILE_PATH="$DOC_ROOT/PRD.md"
    if ! grep -q "## 2. 目标与非目标" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- PRD 缺少「目标与非目标」章节"
    fi
    if ! grep -q "### 非目标" "$FILE_PATH" && ! grep -q "### Won't Have" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- PRD 缺少「非目标」定义"
    fi
    if ! grep -q "## 6. 成功指标" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- PRD 缺少「成功指标」"
    fi
    if ! grep -q "Must Have" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- PRD 功能需求未分优先级"
    fi
    ;;
  SPEC.md)
    FILE_PATH="$DOC_ROOT/SPEC.md"
    if ! grep -q "## 2. 架构设计" "$FILE_PATH" && ! grep -q "## 架构设计" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- SPEC 缺少「架构设计」章节"
    fi
    if ! grep -q "## 3. 技术选型" "$FILE_PATH" && ! grep -q "## 技术选型" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- SPEC 缺少「技术选型」章节"
    fi
    if ! grep -q "理由" "$FILE_PATH" && ! grep -q "原因" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- SPEC 技术选型可能缺少理由说明"
    fi
    if ! grep -q "## 4. 数据模型" "$FILE_PATH" && ! grep -q "## 数据模型" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- SPEC 缺少「数据模型」章节"
    fi
    ;;
  TASK.md)
    FILE_PATH="$DOC_ROOT/TASK.md"
    if ! grep -q "Done when" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- TASK 缺少 Done when 验收标准"
    fi
    if grep -q "Done when：完成" "$FILE_PATH" || grep -q "Done when：实现" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- TASK 存在模糊的 Done when（'完成'或'实现'不是有效标准）"
    fi
    ;;
  TEST.md)
    FILE_PATH="$DOC_ROOT/TEST.md"
    if ! grep -q "测试用例" "$FILE_PATH" && ! grep -q "| ID" "$FILE_PATH"; then
      ISSUES="$ISSUES\n- TEST 缺少具体测试用例"
    fi
    ;;
esac

if [ -n "$ISSUES" ]; then
  echo "[dev-flow] 阶段完成检查发现问题："
  echo -e "$ISSUES"
  echo ""
  echo "请补充以上内容后再推进到下一阶段。"
fi
