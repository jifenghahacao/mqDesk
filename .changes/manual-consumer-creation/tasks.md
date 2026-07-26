# Tasks：MQDesk 手动消费者创建（Consumer Studio）

> 原则：每个任务只改一个功能点，完成后可独立验证。**禁止在人工确认前写任何代码。**

---

## 阶段一：Rust 核心模型与消费者管理器

### Task 1.1 新增消费者相关数据模型
- **目标**：定义消费者配置、状态、消息、过滤条件模型。
- **改动文件**：
  - 改 `src-tauri/crates/core/src/models.rs`
- **实现要点**：
  - 新增 `ConsumerFilter`、`HeaderFilter`、`ManualConsumerConfig`、`ManualConsumer`、`ConsumerMessage`。
  - 所有字段支持序列化/反序列化，兼容前端表单。
- **验收**：
  - `cd src-tauri && cargo check` 无错误。

### Task 1.2 实现消费者管理器
- **目标**：管理 AMQP 消费者生命周期和消息过滤。
- **改动文件**：
  - 新增 `src-tauri/crates/core/src/rabbit/consumer_manager.rs`
  - 改 `src-tauri/crates/core/src/rabbit/mod.rs`
- **实现要点**：
  - `ConsumerManager` 使用 `HashMap<String, ManualConsumerRuntime>` 维护消费者。
  - 提供 `create`、`start`、`pause`、`resume`、`destroy`、`list`、`list_messages`、`ack_message`、`clear_messages` 方法。
  - 消息过滤在 Rust 端执行，缓冲区限制 500 条。
  - 手动 Ack 模式下，销毁时 nack with requeue=true。
- **验收**：
  - `cargo test` 新增测试：状态转换、过滤函数、缓冲区溢出。

### Task 1.3 AppState 集成 ConsumerManager
- **目标**：让 Tauri 命令能访问消费者管理器。
- **改动文件**：
  - 改 `src-tauri/crates/core/src/lib.rs` 或 `src-tauri/src/state.rs`（根据现有 AppState 位置）
- **实现要点**：
  - 在 `AppState` 中新增 `consumer_manager: ConsumerManager`。
  - 确保 `AppState` 初始化时同时初始化管理器。
- **验收**：
  - `cargo check` 通过。

---

## 阶段二：Tauri 命令层

### Task 2.1 新增消费者相关命令
- **目标**：暴露消费者生命周期命令给前端。
- **改动文件**：
  - 改 `src-tauri/src/commands/consumer.rs`
  - 改 `src-tauri/src/commands/mod.rs`
  - 改 `src-tauri/src/lib.rs`
- **实现要点**：
  - 命令签名同 design.md 第 2.2 节。
  - 命令层只做参数传递和错误封装，业务逻辑调用 `state.consumer_manager`。
  - 未连接 RabbitMQ 时返回 `AppError::NotConnected`。
- **验收**：
  - `cargo test` 通过。
  - 命令已在 `lib.rs` 注册。

---

## 阶段三：前端页面与组件

### Task 3.1 API 封装
- **目标**：前端可调用消费者命令。
- **改动文件**：
  - 改 `src/lib/api.js`
- **实现要点**：
  - 新增 `createConsumer`、`startConsumer`、`pauseConsumer`、`resumeConsumer`、`destroyConsumer`、`listConsumers`、`listConsumerMessages`、`ackMessage`、`clearConsumerMessages`。
- **验收**：
  - `npm test` 不因此报错。

### Task 3.2 消费者配置表单组件
- **目标**：实现 ConsumerForm。
- **改动文件**：
  - 新增 `src/components/ConsumerForm.jsx`
- **实现要点**：
  - 名称输入、队列下拉（调用 `listQueues`）、同步/异步单选、预取值数字输入、Ack 模式（默认手动 Ack / 预览模式，可选自动 Ack / 真实消费模式）。
  - payload 过滤：类型下拉（contains/equals/regex）+ 值输入。
  - headers 过滤：动态键值对列表。
  - 表单验证：名称必填、名称唯一、队列必选、正则可合法。
- **验收**：
  - Vitest 覆盖表单验证和提交。

### Task 3.3 消费者列表组件
- **目标**：展示消费者卡片并控制生命周期。
- **改动文件**：
  - 新增 `src/components/ConsumerList.jsx`
- **实现要点**：
  - 每个卡片展示：名称、队列、模式、Ack 模式、状态、消费数量。
  - 根据状态显示不同操作按钮：待启动→开始；运行中→暂停/销毁；已暂停→继续/销毁；错误→重试/销毁。
  - 销毁前弹出确认框。
- **验收**：
  - Vitest 覆盖状态渲染和按钮行为。

### Task 3.4 已消费消息列表组件
- **目标**：展示、展开、确认、清空消息。
- **改动文件**：
  - 新增 `src/components/ConsumerMessageList.jsx`
- **实现要点**：
  - 表格展示：序号、时间、路由键、payload 摘要、headers 摘要、Ack 状态。
  - 点击行展开完整 payload、headers、exchange、redelivered。
  - 手动 Ack 模式下显示「确认」按钮。
  - 顶部显示「清空列表」按钮。
- **验收**：
  - Vitest 覆盖列表渲染、展开、确认、清空。

### Task 3.5 ConsumerStudioView 主页面
- **目标**：整合表单、消费者列表、消息列表。
- **改动文件**：
  - 新增 `src/views/ConsumerStudioView.jsx`
- **实现要点**：
  - 页面头部标题 + 刷新按钮。
  - 左侧配置表单，右侧消费者卡片 + 消息列表。
  - 无连接时展示空状态。
  - `setInterval` 每 2 秒刷新消费者状态和消息列表。
- **验收**：
  - Vitest 覆盖空状态、列表刷新、创建消费者后状态更新。

### Task 3.6 路由与导航
- **目标**：让用户能进入 ConsumerStudio。
- **改动文件**：
  - 改 `src/app.jsx`
  - 改 `src/components/Sidebar.jsx`
  - 改 `src/lib/api.js`（如需）
- **实现要点**：
  - `Sidebar` 新增「消费者工作室」导航项。
  - `app.jsx` 注册 `consumer-studio` 路由，渲染 `ConsumerStudioView`。
- **验收**：
  - 点击导航能进入页面，`npm test` 通过。

---

## 阶段四：样式与帮助提示

### Task 4.1 液态玻璃样式
- **目标**：页面风格与现有系统一致。
- **改动文件**：
  - 改 `src/styles/glass.css`
- **实现要点**：
  - 新增 `.consumer-studio`、`.consumer-form`、`.consumer-card`、`.consumer-message-list` 等类。
  - 复用 `var(--glass)`、`var(--hairline)`、`var(--r-md)` 等变量。
  - 响应式：小屏时左侧面板折叠到上方。
- **验收**：
  - `npm run build` 成功，Biome 检查通过。

### Task 4.2 帮助提示
- **目标**：降低用户理解成本。
- **改动文件**：
  - 改 `src/components/ConsumerForm.jsx`
- **实现要点**：
  - 在「同步/异步」「自动/手动 Ack」「预取值」等字段旁添加 Tooltip 或说明文字。
  - 空状态时提示如何创建第一个消费者。
- **验收**：
  - 鼠标悬停可见帮助文本。

---

## 阶段五：测试与门禁

### Task 5.1 Rust 单元测试
- **目标**：验证消费者管理器核心逻辑。
- **改动文件**：
  - 新增/改 `src-tauri/crates/core/src/rabbit/consumer_manager.rs` 内 `#[cfg(test)]` 模块
- **验收**：
  - `cd src-tauri && cargo test` 全部通过。

### Task 5.2 前端组件测试
- **目标**：验证 UI 行为。
- **改动文件**：
  - 新增 `src/tests/ConsumerForm.test.jsx`
  - 新增 `src/tests/ConsumerList.test.jsx`
  - 新增 `src/tests/ConsumerMessageList.test.jsx`
  - 新增 `src/tests/ConsumerStudioView.test.jsx`
- **验收**：
  - `npm test` 全部通过。

### Task 5.3 全量门禁
- **目标**：保证代码质量。
- **执行方式**：
  - 运行 `python tooling/checks.py guard`。
- **验收**：
  - build / lint / typecheck / test 全部成功，无新增 warning。
