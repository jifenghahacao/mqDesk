# RabbitMQ 生产可用性增强（Phase 1）开发任务

> 原则：每个任务只改一个功能点，便于 review、回滚与测试。  
> 所有任务完成后执行「门禁检查」章节。

---

## 第一阶段：后端核心能力

### 任务 1.1：新增数据模型

**目标**：在 `mqdesk-core` 中定义本阶段所需的新模型。  
**修改文件**：

- `src-tauri/crates/core/src/models.rs`

**原子改动**：

- [ ] 新增 `BindingInfo` 结构体。
- [ ] 新增 `ConnectionInfo` 结构体。
- [ ] 新增 `ChannelInfo` 结构体。
- [ ] 新增 `Pagination` 与 `Paginated<T>` 结构体。
- [ ] 新增 `QueueRefreshEvent` 结构体。

**验收**：

- `cargo check -p mqdesk-core` 通过。
- 新增模型可通过 `serde_json::to_string` 序列化，字段与 design.md 一致。

**禁止触碰**：`src-tauri/crates/core/src/health.rs`、`src-tauri/crates/core/src/crypto.rs`

---

### 任务 1.2：实现 Management API 限流器

**目标**：保护 RabbitMQ Management API 不被高频请求压垮。  
**修改文件**：

- 新增 `src-tauri/crates/core/src/rabbit/rate_limiter.rs`
- `src-tauri/crates/core/src/rabbit/mod.rs`

**原子改动**：

- [ ] 实现 `ManagementRateLimiter`，默认 20 permit/s。
- [ ] 提供 `acquire().await` 方法。
- [ ] 在 `mod.rs` 中导出。

**验收**：

- 单元测试：1 秒内连续请求 25 次，前 20 次立即通过，后 5 次在 1 秒内陆续通过。
- `cargo test -p mqdesk-core rate_limiter` 通过。

**禁止触碰**：`src-tauri/crates/core/src/rabbit/management.rs`（限流器不依赖它）。

---

### 任务 1.3：ManagementClient 接入限流与缓存框架

**目标**：让 `ManagementClient` 的所有读取调用先经过限流，并具备降级缓存能力。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`

**原子改动**：

- [ ] 在 `ManagementClient` 中持有 `Arc<ManagementRateLimiter>`。
- [ ] 新增 `get_json_with_fallback<T>()` 方法（先请求，失败时返回缓存）。
- [ ] 为 `get_overview_stats`、`list_queues`、`list_nodes`、`list_consumers` 等读取接口接入缓存字段。

**验收**：

- 单元测试：模拟首次调用成功，验证缓存被写入；模拟第二次调用失败，验证返回缓存且 `is_stale=true`。
- `cargo test -p mqdesk-core management_client` 通过。

**禁止触碰**：写操作方法（`create_queue`、`delete_queue` 等）保持原样，不接入缓存。

---

### 任务 1.4：实现队列 Purge

**目标**：提供清空队列消息的 API。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`
- `src-tauri/src/commands/queue.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`

**原子改动**：

- [x] 在 `management.rs` 中实现 `purge_queue(name)`，调用 `DELETE /api/queues/{vhost}/{name}/contents`。
- [x] 在 `commands/queue.rs` 中新增 `purge_queue` Tauri 命令，记录审计日志。
- [x] 注册命令。

**验收**：

- [x] 单元测试：mock server 收到正确的 DELETE 请求，返回 204 时函数成功。
- [x] 单元测试：mock server 返回 403 时函数返回 `AuthFailed` 或对应错误。
- [x] `cargo test -p mqdesk-core purge` 通过。

**禁止触碰**：`src-tauri/crates/core/src/rabbit/consumer_manager.rs`

---

### 任务 1.5：实现 Bindings 查询与删除

**目标**：支持查看队列绑定并解绑。  
**修改文件**：

- `src-tauri/crates/core/src/models.rs`
- `src-tauri/crates/core/src/rabbit/management.rs`
- 新增 `src-tauri/src/commands/binding.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`

**原子改动**：

- [x] 在 `models.rs` 中为 `BindingInfo` 增加 `Deserialize`，使其可作为 Tauri 命令参数。
- [x] 在 `management.rs` 中实现 `list_queue_bindings` 与 `delete_binding`。
- [x] 新增 `commands/binding.rs`，实现 `list_queue_bindings` 与 `delete_queue_binding` 命令。
- [x] 注册命令。

**验收**：

- [x] 单元测试：mock `/api/queues/{vhost}/{name}/bindings` 返回 2 条绑定，解析结果正确。
- [x] 单元测试：mock 删除绑定返回 204，验证 URL 编码（含 `/` vhost 与特殊字符）。
- [x] `cargo test -p mqdesk-core binding` 通过。

**禁止触碰**：`src-tauri/crates/core/src/storage.rs`

---

### 任务 1.6：实现 Connections 列表

**目标**：支持查看 RabbitMQ 连接摘要。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`
- `src-tauri/src/commands/connection.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- 新增 `src-tauri/crates/core/tests/management_connection_test.rs`

**原子改动**：

- [x] 在 `management.rs` 中实现 `list_connections(pagination)`，调用 `/api/connections/{vhost}?page=&page_size=`。
- [x] 扩展 `ManagementConnection` 响应结构体，包含 `peer_host`、`peer_port`、`protocol`、`connected_at`、`channels`、`state`。
- [x] 在 `commands/connection.rs` 中新增 `list_rabbit_connections` 命令（避免与已有的本地连接 `list_connections` 命令冲突）。
- [x] 注册命令。

**验收**：

- [x] 单元测试：mock `/api/connections` 返回 3 条连接，解析后 `peer_address`、`connected_seconds` 正确。
- [x] 单元测试：分页参数 `page=2&page_size=10` 正确附加到 URL。
- [x] `cargo test -p mqdesk-core connection` 通过。

**调整说明**：因 `list_connections` 已被本地连接配置占用，新增命令名为 `list_rabbit_connections`，前端对应 `listRabbitConnections`。

**禁止触碰**：`src-tauri/src/commands/connection.rs` 中已有的连接 CRUD 命令。

---

### 任务 1.7：实现 Channels 列表

**目标**：支持按连接查看信道。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`
- 新增 `src-tauri/src/commands/channel.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- 新增 `src-tauri/crates/core/tests/management_channel_test.rs`

**原子改动**：

- [x] 在 `management.rs` 中实现 `list_channels(connection_name, pagination)`，调用 `/api/channels/{vhost}?page=&page_size=`。
- [x] 扩展 `MessageStats` 增加 `ack_details`，新增 `ManagementChannel` 响应结构体。
- [x] 新增 `commands/channel.rs`，实现 `list_channels` 命令。
- [x] 注册命令。

**验收**：

- [x] 单元测试：mock `/api/channels` 返回 5 条信道，按 `connection_name` 过滤后返回 2 条。
- [x] 单元测试：速率字段解析为浮点数，不丢失精度。
- [x] `cargo test -p mqdesk-core channel` 通过。

---

### 任务 1.8：修复 `list_consumers` N+1 查询

**目标**：消除消费者列表的 N+1 查询。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`

**原子改动**：

- [ ] 修改 `list_consumers`，先批量拉取 `/api/queues` 获取所有队列的 `deliver_get_details.rate`。
- [ ] 用 `HashMap<queue_name, rate>` 替换循环中的 `get_queue` 调用。
- [ ] 保持返回类型 `Vec<ConsumerInfo>` 不变。

**验收**：

- 单元测试：100 个队列 + 50 个消费者场景下，HTTP 调用次数 ≤ 5 次。
- 单元测试：`message_rate` 与队列速率一致。
- `cargo test -p mqdesk-core consumer` 通过。

**禁止触碰**：`src-tauri/src/commands/consumer.rs`（命令层不变）。

---

### 任务 1.9：队列列表支持分页

**目标**：让 `list_queues` 支持服务端分页。  
**修改文件**：

- `src-tauri/crates/core/src/rabbit/management.rs`
- `src-tauri/src/commands/queue.rs`
- `src-tauri/src/commands/alert.rs`
- `src-tauri/src/commands/overview.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/crates/core/tests/smoke_test.rs`
- 新增 `src-tauri/crates/core/tests/management_queue_pagination_test.rs`

**原子改动**：

- [x] 修改 `ManagementClient::list_queues` 签名，增加 `pagination: &Pagination`；URL 附加 `page` 与 `page_size`。
- [x] 保留旧的 `list_queues` Tauri 命令返回 `Vec<QueueSummary>`，避免前端立即崩溃。
- [x] 新增 `list_queues_paginated` 命令返回 `Paginated<QueueSummary>`，供任务 2.3 切换使用。
- [x] 同步更新 `alert.rs`、`overview.rs`、`smoke_test.rs` 中的调用。

**验收**：

- [x] 单元测试：mock 验证请求 URL 包含 `page=1&page_size=50`。
- [x] 单元测试：返回 `Paginated` 的 `total`、`items`、`page`、`page_size` 字段正确。
- [x] `cargo test -p mqdesk-core queue_pagination` 通过。

**注意**：为避免破坏现有前端，新增命令 `list_queues_paginated`；任务 2.3 将前端从 `list_queues` 切换到新命令。

---

### 任务 1.10：实现后台自动刷新任务

**目标**：周期性拉取队列状态并通过 Tauri Event 推送。  
**修改文件**：

- 新增 `src-tauri/crates/core/src/rabbit/refresh_task.rs`
- `src-tauri/crates/core/src/rabbit/mod.rs`
- `src-tauri/crates/core/src/state.rs`
- `src-tauri/src/lib.rs`（启动时注入 AppHandle）

**原子改动**：

- [x] 实现 `RefreshTask` 结构体，支持 start/stop/set_interval/set_enabled。
- [x] 在 `AppState` 中持有 `RefreshTask`。
- [x] 在 `set_active` 时启动任务，`clear_active` 时停止任务。
- [x] 任务循环中调用 `app.emit_to(window_label, "queue-refreshed", event)` 与 `management-stale`。

**验收**：

- [x] 单元测试：模拟时间推进，验证任务按设定周期触发。
- [x] 集成测试：启动 Tauri 后，队列状态变化能在 5 秒内收到事件。
- [x] `cargo test -p mqdesk-core refresh_task` 通过。

**禁止触碰**：`src-tauri/crates/core/src/storage.rs`

---

## 第二阶段：前端适配与新增视图

### 任务 2.1：前端 API 封装

**目标**：在 `src/lib/api.js` 中暴露新命令与事件监听。  
**修改文件**：

- `src/lib/api.js`

**原子改动**：

- [ ] 新增 `purgeQueue(name)`。
- [ ] 新增 `listQueueBindings(queueName)`、`deleteQueueBinding(...)`。
- [ ] 新增 `listConnections(pagination)`、`listChannels(connectionName, pagination)`。
- [ ] 新增 `listenQueueRefreshed(callback)`、`listenManagementStale(callback)`。
- [ ] 修改 `listQueues(filter, pagination)` 以适配新的分页返回结构。

**验收**：

- 单元测试：mock Tauri `invoke`，验证调用名称与参数正确。
- `npm test` 通过。

**禁止触碰**：`src/lib/api-mock.js` 如需更新可同步，但不强制。

---

### 任务 2.2：新增虚拟滚动表格组件

**目标**：为长列表提供高性能渲染。  
**修改文件**：

- 新增 `src/components/VirtualTable.jsx`
- `src/styles/glass.css`（补充样式）

**原子改动**：

- [ ] 实现固定行高的 `VirtualTable`，支持 `rowHeight`、`buffer`、`renderRow` props。
- [ ] 计算可见区间与 buffer，仅渲染可见行。
- [ ] 补充 hover、选中行样式。

**验收**：

- 单元测试：传入 500 条数据，渲染后 DOM 中行数 ≤ 30。
- 单元测试：滚动后渲染区间更新正确。
- `npm test` 通过。

**禁止触碰**：`src/styles/tokens.css`

---

### 任务 2.3：队列列表接入分页与虚拟滚动

**目标**：改造 `QueuesView` 以支持分页和虚拟滚动。  
**修改文件**：

- `src/views/QueuesView.jsx`
- `src/components/VirtualTable.jsx`（如需调整 props）

**原子改动**：

- [ ] 修改 `listQueues` 调用，传入 `pagination`。
- [ ] 用 `Paginated<QueueSummary>` 解构 `items` 与 `total`。
- [ ] 用 `VirtualTable` 替换原生 `<table>` 渲染。
- [ ] 新增分页控件（上一页/下一页/页码/每页条数）。

**验收**：

- E2E/人工：500 队列场景下首屏加载 ≤ 1s，滚动帧率 ≥ 30fps。
- `npm run build` 通过。

**禁止触碰**：`src/views/QueueDetailView.jsx`

---

### 任务 2.4：队列详情新增绑定 Tab 与清空按钮

**目标**：在队列详情页展示绑定并支持 Purge。  
**修改文件**：

- `src/views/QueueDetailView.jsx`
- 新增 `src/components/BindingRow.jsx`
- `src/styles/glass.css`

**原子改动**：

- [ ] 在详情页新增「绑定」Tab。
- [ ] 调用 `listQueueBindings` 加载绑定列表。
- [ ] 使用 `BindingRow` 渲染每条绑定，提供解绑按钮（二次确认）。
- [ ] 在操作栏新增「清空队列」按钮（二次确认）。

**验收**：

- 人工：打开队列详情，绑定列表展示正确；点击解绑后列表刷新。
- 人工：点击清空队列后，Ready/Total 归零。
- `npm run build` 通过。

**禁止触碰**：`src/components/QueueFormModal.jsx`

---

### 任务 2.5：新增连接与信道视图

**目标**：实现 Connections 列表与 Channels 展开面板。  
**修改文件**：

- 新增 `src/views/ConnectionsView.jsx`
- 新增 `src/components/ConnectionRow.jsx`
- 新增 `src/components/ChannelRow.jsx`
- `src/app.jsx`
- `src/components/Sidebar.jsx`
- `src/styles/glass.css`

**原子改动**：

- [ ] 实现 `ConnectionsView`，展示连接列表。
- [ ] 点击连接行展开 `ChannelRow` 列表（调用 `listChannels`）。
- [ ] 在 `app.jsx` 中注册 `connections` 视图。
- [ ] 在 `Sidebar` 新增「连接」导航项。

**验收**：

- 人工：导航到「连接」视图，列表展示正确；点击连接展开信道。
- `npm run build` 通过。

**禁止触碰**：`src/views/ConsumersView.jsx`

---

### 任务 2.6：设置页新增自动刷新开关

**目标**：让用户控制自动刷新行为。  
**修改文件**：

- `src/views/SettingsView.jsx`
- 新增 `src/components/RefreshToggle.jsx`

**原子改动**：

- [ ] 实现 `RefreshToggle` 组件：开关 + 周期选择（5s/15s/30s/60s）。
- [ ] 在 `SettingsView` 中嵌入组件。
- [ ] 将配置持久化到 `localStorage`（前端本地即可，无需后端存储）。

**验收**：

- 单元测试：切换开关和周期，localStorage 值正确。
- `npm test` 通过。

**禁止触碰**：`src-tauri/crates/core/src/storage.rs`

---

### 任务 2.7：总览与队列列表订阅刷新事件

**目标**：让关键视图自动更新。  
**修改文件**：

- `src/views/OverviewView.jsx`
- `src/views/QueuesView.jsx`
- 新增全局 stale banner 组件（或内嵌在 `app.jsx`）

**原子改动**：

- [ ] 在 `OverviewView` 的 `useEffect` 中监听 `queue-refreshed`，更新 state。
- [ ] 在 `QueuesView` 中监听同一事件，仅在当前第 1 页时更新（避免翻页时被重置）。
- [ ] 监听 `management-stale`，展示/隐藏全局降级 banner。

**验收**：

- E2E/人工：在 RabbitMQ 中制造队列堆积，5 秒内前端数字自动更新。
- E2E/人工：断开 Management API 网络，3 秒内展示 stale banner。
- `npm run build` 通过。

**禁止触碰**：`src-tauri/src/lib.rs`

---

## 第三阶段：联调、测试与门禁

### 任务 3.1：后端门禁

**执行命令**：

```bash
cd src-tauri
cargo check
cargo test -p mqdesk-core
```

**验收**：

- [ ] `cargo check` 无错误、无警告（允许已存在的 warning）。
- [ ] `cargo test -p mqdesk-core` 全部通过。

---

### 任务 3.2：前端门禁

**执行命令**：

```bash
npm run lint
npm test
npm run build
```

**验收**：

- [ ] `npm run lint` 无错误。
- [ ] `npm test` 全部通过。
- [ ] `npm run build` 成功生成 `dist/`。

---

### 任务 3.3：端到端验证

**环境**：本地启动 RabbitMQ（可用 `scripts/rabbitmq-cluster.yml` 单节点）。  
**场景**：

- [ ] 创建 500 个队列，验证队列列表首屏加载 ≤ 1s。
- [ ] 向某队列发送 1000 条消息，验证 Purge 后消息数为 0。
- [ ] 创建绑定并验证列表展示，验证解绑成功。
- [ ] 创建 AMQP 消费者，验证 Connections/Channels 视图展示正确。
- [ ] 开启自动刷新 5s，验证堆积数字自动更新。
- [ ] 模拟 Management API 故障，验证 stale banner 出现。

---

## 禁止触碰文件清单（全阶段通用）

| 文件/目录 | 原因 |
|-----------|------|
| `src/styles/tokens.css` | ADR 003：核心 Token 不可变。 |
| `legacy/` | ADR 004：归档冻结。 |
| `src-tauri/crates/core/src/health.rs` | 健康度算法本次不变。 |
| `src-tauri/crates/core/src/crypto.rs` | 加密逻辑不涉及。 |
| `src-tauri/crates/core/src/storage.rs` | 本次不新增持久化树。 |
| `src-tauri/crates/core/src/rabbit/consumer_manager.rs` | 手动消费者不做改动。 |
| `src-tauri/crates/core/src/rabbit/publisher.rs` | AMQP 发布不做连接池改造。 |
| `src-tauri/src/main.rs` | 入口保持现状。 |
| `src/main.jsx` | 入口保持现状。 |
| `src-tauri/tauri.conf.json` | 不修改窗口/Bundle 配置。 |

---

## 任务依赖图

```text
1.1 模型
  ├── 1.2 限流器
  │     └── 1.3 ManagementClient 接入限流/缓存
  │           ├── 1.4 Purge
  │           ├── 1.5 Bindings
  │           ├── 1.6 Connections
  │           ├── 1.7 Channels
  │           ├── 1.8 N+1 修复
  │           ├── 1.9 队列分页
  │           └── 1.10 后台刷新任务
  │
2.1 前端 API 封装
  ├── 2.2 VirtualTable
  │     └── 2.3 队列列表分页/虚拟滚动
  ├── 2.4 队列详情绑定/Purge
  ├── 2.5 连接/信道视图
  ├── 2.6 设置自动刷新
  └── 2.7 总览/队列订阅事件
        └── 依赖 1.10

3.1 后端门禁
3.2 前端门禁
3.3 E2E 验证
```
