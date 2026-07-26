# 系统架构与分层约束

> 只写硬性边界。存量代码不强制改造，新代码必须遵守。

## 1. Tauri 命令层不写业务规则

- **规则描述**：`src-tauri/src/commands/` 下的函数只做参数接收、`State` 读取、`invoke` 转发和 `AppResult` 返回，不实现任何业务规则。
- **违规风险**：业务逻辑散落在命令层，导致 `mqdesk-core` 无法独立测试，前端也失去统一后端入口。
- **可执行校验**：
  ```powershell
  # 命令层文件不得出现mqdesk_core外的具体业务实现
  cd src-tauri
  cargo check
  cargo test --lib
  ```

## 2. mqdesk-core 不依赖 Tauri

- **规则描述**：`src-tauri/crates/core/` 不能依赖 `tauri`、`tauri-plugin-*` 或任何 WebView 相关 crate。
- **违规风险**：核心库与桌面壳耦合，无法独立运行测试，也无法在未来复用为 CLI/服务。
- **可执行校验**：
  ```powershell
  cd src-tauri/crates/core
  cargo check
  ```
  若出现 tauri 依赖，编译失败。

## 3. 前端不直接访问 RabbitMQ

- **规则描述**：前端代码只能调用 `src/lib/api.js` 封装的 Tauri 命令，禁止直接引入 `reqwest`、`lapin`、`amqplib` 等网络/AMQP 库，禁止直接发起 HTTP/WebSocket 到 RabbitMQ。
- **违规风险**：密码、AMQP URL、Management 凭据暴露给前端，违反安全边界。
- **可执行校验**：
  ```powershell
  # 检查前端是否直接引入网络/AMQP库
  Select-String -Path "src/**/*.js","src/**/*.jsx" -Pattern "reqwest|lapin|amqplib|amqp|axios|fetch\s*\(" -ErrorAction SilentlyContinue
  ```

## 4. 凭据不出 Rust 进程

- **规则描述**：密码、解密后的 AMQP URL、Management Auth Header 不得序列化到前端，不得在日志中明文输出。
- **违规风险**：凭据泄露，用户本地安全受损。
- **可执行校验**：
  ```powershell
  # 检查核心库是否导出含密码的模型字段
  Select-String -Path "src-tauri/crates/core/src/**/*.rs" -Pattern "pub\s+password|password:\s*String" -ErrorAction SilentlyContinue
  # 检查命令层返回Connection时是否清空密码
  Select-String -Path "src-tauri/src/commands/connection.rs" -Pattern "password\s*=\s*String::new" -ErrorAction SilentlyContinue
  ```

## 5. legacy/ 目录冻结

- **规则描述**：`legacy/` 目录仅作旧 Go/React 项目历史参考，禁止修改其中任何文件。
- **违规风险**：污染历史参考，可能导致旧架构与新架构混淆。
- **可执行校验**：
  ```powershell
  git diff --name-only HEAD | Select-String "^legacy/"
  ```
  任何 PR 若包含 `legacy/` 路径改动，禁止合并。

## 6. 设计 Token 不可变

- **规则描述**：`src/styles/tokens.css` 中的核心变量 `--primary`、`--glass`、`--sh-1/2/3` 等不得修改、移除或重命名；新增 Token 需同步到 `DesignSystemManifest.md`。
- **违规风险**：破坏整站液态玻璃视觉一致性。
- **可执行校验**：
  ```powershell
  # 检查tokens.css是否仍包含核心变量
  Select-String -Path "src/styles/tokens.css" -Pattern "--primary|--glass|--sh-1|--sh-2|--sh-3" -ErrorAction SilentlyContinue
  ```
