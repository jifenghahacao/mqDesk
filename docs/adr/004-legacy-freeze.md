# ADR 004: Legacy 目录冻结

- 状态：Accepted
- 日期：2026-07-26

## 决策

1. `legacy/` 目录保存旧 Go/React 项目（RabbitConsumerHub 原架构），仅作历史参考。
2. 禁止修改 `legacy/` 内任何文件，包括代码、配置、文档和脚本。
3. 新功能、新修复必须在 `src/` 和 `src-tauri/` 中实现。

## 后果

- 对 legacy 的任何改动都会污染历史参考并可能导致构建/理解混乱。
