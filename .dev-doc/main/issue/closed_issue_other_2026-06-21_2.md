---
source: other
nums: 1
---

- [x] ISSUE-I002：Codex hook 输出需要与其他 agent 隔离
  - severity: P1
  - location：dow/src/hooks
  - description：Codex hook 需要把 agent context 和用户可见信息分层，避免给 agent 的上下文直接显示给用户。特殊行为必须只在 `--codex-hook` 下生效，不能破坏 Claude/Kiro 现有输出。
  - reproduce：触发 dev-flow Codex hook，用户界面看到本应只给 agent 使用的上下文或调试信息。
  - fix：审计 `--codex-hook` 路径，只在 Codex hook 协议输出机器可读内容；非 Codex 路径保持原有人类输出。
