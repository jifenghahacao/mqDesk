# ADR 002: 安全边界 — 密钥不暴露前端

- 状态：Accepted
- 日期：2026-07-26

## 决策

1. 所有 RabbitMQ 通信（Management HTTP API 与 AMQP）必须在 Rust 后端执行。
2. 前端唯一入口为 `src/lib/api.js` 中的 `invoke` 封装；禁止直接引入 `reqwest`、`lapin` 或任何网络/AMQP 库。
3. 密码、AMQP URL、Management 凭据不得离开 Rust 进程。
4. 本地加密由 `src-tauri/crates/core/src/crypto.rs` 实现，密文存储于 sled。

## 后果

- 绕过 Tauri 命令直接从前端调用网络/AMQP 属于安全违规，会导致密钥泄露。
