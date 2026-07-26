# 通用编码规范

> 存量代码不强制改造，新代码必须遵守。每条规则附带可执行校验方式。

## 1. Rust 错误统一走 AppResult

- **规则描述**：`mqdesk-core` 内部函数返回 `AppResult<T>`，不得使用裸 `Result<T, Box<dyn Error>>` 或 `unwrap()` 处理业务错误。
- **违规风险**：错误类型不统一，前端收到不可读的错误信息，且容易 panic。
- **可执行校验**：
  ```powershell
  cd src-tauri/crates/core
  cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used
  ```
  注：初始化、托盘图标解码等真正不可恢复场景允许 `expect`，需在代码注释中说明。

## 2. Tauri 命令必须返回 AppResult

- **规则描述**：`src-tauri/src/commands/` 下的所有 `#[tauri::command]` 函数必须返回 `AppResult<T>`，由 `error.rs` 统一序列化。
- **违规风险**：前端无法统一处理错误，异常直接抛到 UI。
- **可执行校验**：
  ```powershell
  cd src-tauri
  cargo check
  cargo test
  ```

## 3. 新代码必须包含测试

- **规则描述**：新增 Rust 模块需附带单元测试；新增前端组件/视图需附带 Vitest 测试；纯 UI 样式除外。
- **违规风险**：回归无保障，重构困难。
- **可执行校验**：
  ```powershell
  npm test
  cd src-tauri && cargo test
  ```

## 4. 前端 API 调用统一入口

- **规则描述**：前端调用后端必须通过 `src/lib/api.js` 中的函数，禁止在视图/组件中直接 `invoke('command_name', ...)`。
- **违规风险**：调用点分散，参数变更时难以追踪。
- **可执行校验**：
  ```powershell
  Select-String -Path "src/**/*.js","src/**/*.jsx" -Pattern "invoke\s*\(" -ErrorAction SilentlyContinue | Where-Object { $_.Path -notmatch "api\.js" }
  ```

## 5. 密码加密必须使用 crypto.rs

- **规则描述**：任何涉及密码存储/读取的代码必须调用 `mqdesk_core::crypto::encrypt/decrypt`，禁止自行实现加密或明文保存。
- **违规风险**：凭据泄露或加密方案不一致。
- **可执行校验**：
  ```powershell
  Select-String -Path "src-tauri/**/*.rs" -Pattern "encrypt\(|decrypt\(" -ErrorAction SilentlyContinue | Where-Object { $_.Path -notmatch "crypto\.rs" }
  ```

## 6. CSS 变量必须来自 tokens.css

- **规则描述**：样式中使用的颜色、间距、圆角、阴影等必须引用 `tokens.css` 的 CSS 变量，禁止硬编码色值。
- **违规风险**：视觉不一致，主题切换失效。
- **可执行校验**：
  ```powershell
  # 扫描glass.css外的样式文件中的硬编码色值（允许过渡/动画等特殊值）
  Select-String -Path "src/**/*.css" -Pattern "#(?i)[0-9a-f]{3,8}\b|rgb\(|rgba\(" -ErrorAction SilentlyContinue | Where-Object { $_.Path -notmatch "tokens\.css" }
  ```

## 7. 文件与目录命名

- **规则描述**：
  - Rust：模块、函数、变量用 `snake_case`，类型用 `PascalCase`。
  - 前端组件/视图文件用 `PascalCase.jsx`，工具库用 `camelCase.js`。
  - CSS 类名使用 `kebab-case`。
- **违规风险**：命名混乱，跨团队协作成本上升。
- **可执行校验**：
  ```powershell
  # Rust 命名由编译器保证；前端文件命名人工 review
  Get-ChildItem -Path "src/components","src/views","src/lib" | Where-Object { $_.Extension -in ".js",".jsx" }
  ```

## 8. 注释与文档

- **规则描述**：Rust 公共模块必须写中文 `//!` 模块级文档；复杂业务逻辑需加注释。前端复杂函数需写 JSDoc。
- **违规风险**：后续维护者难以理解设计意图。
- **可执行校验**：
  ```powershell
  cd src-tauri/crates/core
  cargo doc --no-deps
  ```
