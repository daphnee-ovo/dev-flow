---
source: other
nums: 1
---

- [x] ISSUE-I001：过期 GitHub latest 缓存导致版本重置后误报更新
  - severity: P1
  - location：dow/src/main.rs
  - description：`dow` 启动时只按缓存的 `latest_remote_version` 判断更新，且先打印旧缓存再后台刷新。版本号重置后，本机旧缓存 `3.8.8` 会在 GitHub latest 已回到 `0.1.x` 时继续提示“新版本可用”。更新判断还缺少 GitHub release `published_at`，无法识别版本 epoch 重置。
  - reproduce：在 `~/.config/dow/config.toml` 中保留过期 `last_version_check` 和 `latest_remote_version = "3.8.8"`，运行任意触发版本检查的 `dow` 命令。
  - fix：缓存写入 `version + published_at + notes`；缓存有效期最多 24 小时，过期后本次命令强制刷新 GitHub latest；刷新失败时清除远端缓存；更新提示统一通过 `version + published_at` 判断。
