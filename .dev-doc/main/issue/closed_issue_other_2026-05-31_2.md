---
source: other
nums: 1
---

- [x] ISSUE-I001：Windows 安装后 dow setup 找不到 bundle 目录
  - severity: P0
  - location：install/install.ps1:8 + dow/src/core/platform.rs:28-35
  - description：install.ps1 将 bundle 安装到 $USERPROFILE\.local\share\dow\bundle\，但 platform.rs 在 Windows 上查找 %LOCALAPPDATA%\dow\bundle\，路径不一致导致 dow setup 报错"插件 bundle 不存在"
  - reproduce：Windows 上运行 install.ps1 后执行 dow setup，提示 bundle 不存在
  - fix：install.ps1 的 DATA_DIR 改为使用 $env:LOCALAPPDATA\dow，与 platform.rs 的 data_dir() 保持一致

