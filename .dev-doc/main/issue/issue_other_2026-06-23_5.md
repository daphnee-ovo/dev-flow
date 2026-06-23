---
source: other
nums: 1
---

- [x] ISSUE-I009：全项目默认语言改为英文
  - severity: P1
  - location：dow/src/ + plugin/
  - description：dow CLI 所有用户可见输出（42个rs文件1224行）和 plugin 命令文档（18个md文件922行）的中文内容改为英文。包括错误信息、提示、注释、文档正文
  - fix：42 个 Rust 文件 + 18 个 plugin markdown 文件全部翻译为英文，零中文残留。编译通过，29/29 fix/validate/scan/status 测试通过

