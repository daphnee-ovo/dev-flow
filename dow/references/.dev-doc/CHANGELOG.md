# CHANGELOG.md 格式规范

## 模板

```markdown
# Changelog

## 2026-05-16
- 14:30 fix-login-bug: 修复登录验证逻辑
- 10:00 implement-auth: 完成认证模块基础结构

## 2026-05-15
- 16:00 init-project: 项目初始化
```

## 追加规则

- `save-changelog` hook（Stop 触发）自动追加一条记录
- 格式：`- HH:MM <topic>: <一句话摘要>`
- 如果当天日期段不存在，先插入 `## YYYY-MM-DD` 行
- topic 从最近 git commit message 推断，fallback 为当前 phase 名称

## 生命周期

- `/iterate` 时自动归档到 `dev-doc/archive.db`（SQLite），然后重置为空（新迭代从空 CHANGELOG 开始）
- inject-context 注入最近一条记录作为上下文
