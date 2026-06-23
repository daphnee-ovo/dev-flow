---
source: other
nums: 1
---

- [x] ISSUE-I007：install.sh 下载不够健壮
  - severity: P2
  - location：install/install.sh:57
  - description：下载无进度显示、无重试、无超时、无文件完整性检查，网络不稳时安装会静默失败
  - fix：download() 增加 3 次重试、connect-timeout 10s、max-time 120s、-s 文件非空校验、进度条显示；失败时清理临时目录并给出明确错误提示

