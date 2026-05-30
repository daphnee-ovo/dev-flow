# TEST.md 格式规范

## 模板

```markdown
# 测试报告

- 执行时间：YYYY-MM-DD HH:MM
- 测试范围：全量 / 指定模块
- 总用例数：N
- 通过：N
- 失败：N

## 失败用例

| 模块 | 用例 | 错误信息 | 关联 issue |
|------|------|----------|-----------|
| auth | test_login_with_invalid_email | AssertionError... | issue_test_2026-05-15_1 |

## 通过模块
- auth（12/12）
- api（8/8）
```

## 说明

- `/test` 执行后产出，记录本次测试结果
- 重新测试时覆盖（不追加）
- 失败用例需关联对应 issue 文件
