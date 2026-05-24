---
source: test
nums: 1
---

- [x] I1：context.sh 空项目缺少"空项目"统一提示
  - severity: P2
  - location：scripts/lib/context.sh:36
  - description：SPEC 第 7 节兼容性要求"空项目（无 src/、无 tests/）时输出'空项目'而非报错"。实际行为：空目录执行退出码为 0、不报错，各部分分别输出"无法自动推断"/"无标准运行入口"/"无标准模块目录"，但缺少一个统一的"空项目"字样提示。功能上不影响使用，但与 SPEC 描述不一致。
  - reproduce：`mkdir -p tmp/empty_proj && bash scripts/lib/context.sh tmp/empty_proj | grep "空项目"`，返回 0 行匹配（应有匹配）
  - fix：在 context.sh 开头添加空项目检测，目录内无文件且无子目录时直接输出"（空项目）"并返回
