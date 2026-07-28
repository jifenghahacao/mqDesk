# RabbitMQ 生产可用性增强（Phase 1）设计文档

## 一、总体架构

```text
┌─────────────────────────────────────────────────────────────┐
│  Frontend (Preact)                                          │
│  ├── Views: QueuesView, QueueDetailView, ConnectionsView,   │
│  │          ChannelsView, SettingsView                      │
│  ├── Components: VirtualTable, BindingRow, ConnectionRow,   │
│  │                ChannelRow, RefreshToggle                 │
│  └── api.js: 新增 purge_queue / list_bindings / ...         │
└─────────────────────────┬───────────────────────────────────┘
                          │ invoke / event
┌─────────────────────────▼───────────────────────────────────┐
│  Tauri Command Layer (src-tauri/src/commands/)              │
│  ├── queue.rs       → 新增 purge_queue                      │
│  ├── binding.rs     → 新增 list_bindings / delete_binding   │
│  ├── connection.rs  → 新增 list_connections                 │
│  ├── channel.rs     → 新增 list_channels                    │
│  ├── overview.rs    → 接入自动刷新事件                      │
│  └── settings.rs    → 新增刷新配置读写（如需要）            │
└─────────────────────────┬───────────────────────────────────┘
                          │ call
┌─────────────────────────▼───────────────────────────────────┐
│  mqdesk-core (src-tauri/crates/core/src/)                   │
│  ├── rabbit/management.rs  → 新增 API + 分页 + 限流         │
│  ├── rabbit/rate_limiter.rs → 新增全局令牌桶               │
│  ├── rabbit/refresh_task.rs → 新增后台刷新任务             │
│  ├── models.rs            → 新增 Binding / Connection /     │
│  │                         Channel 模型                     │
│  ├── state.rs             → 接入刷新任务与缓存              │
│  └── health.rs            → 保持现状，不改动核心算法        │
└─────────────────────────────────────────────────────────────┘
```

## 二、数据模型

### 2.1 新增/扩展 Rust 模型

```rust
// src-tauri/crates/core/src/models.rs

/// 队列绑定
#[derive(Debug, Clone, Serialize)]
pub struct BindingInfo {
    pub source: String,          // 交换机名，空字符串表示默认交换机直接绑定
    pub vhost: String,
    pub destination: String,     // 队列名
    pub destination_type: String,// "queue"
    pub routing_key: String,
    pub arguments: serde_json::Value,
    pub properties_key: String,  // 用于删除绑定的 key
}

/// RabbitMQ 连接摘要
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub peer_host: String,
    pub peer_port: u16,
    pub peer_address: String,    // 聚合字段：host:port
    pub protocol: String,
    pub connected_at: u64,       // ms
    pub connected_seconds: u64,
    pub channel_count: u32,
    pub state: String,           // running / blocked / blocking
}

/// RabbitMQ 信道摘要
#[derive(Debug, Clone, Serialize)]
pub struct ChannelInfo {
    pub name: String,
    pub connection_name: String,
    pub number: u16,
    pub consumer_count: u32,
    pub prefetch_count: u16,
    pub unacked: u64,
    pub publish_rate: f64,
    pub deliver_rate: f64,
    pub ack_rate: f64,
}

/// 分页参数（复用于队列、连接、信道）
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 50 }

/// 分页结果包装
#[derive(Debug, Clone, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}
```

### 2.2 自动刷新事件 payload

```rust
#[derive(Debug, Clone, Serialize)]
pub struct QueueRefreshEvent {
    pub queues: Vec<QueueSummary>,
    pub overall_health: HealthStatus,
    pub alert_count: u64,
    pub is_stale: bool,
}
```

## 三、Management API 封装

### 3.1 新增/修改方法（management.rs）

```rust
impl ManagementClient {
    /// 清空队列
    pub async fn purge_queue(&self, name: &str) -> AppResult<()>;

    /// 列出队列绑定
    pub async fn list_queue_bindings(&self, queue_name: &str) -> AppResult<Vec<BindingInfo>>;

    /// 删除绑定
    pub async fn delete_binding(
        &self,
        exchange: &str,
        queue_name: &str,
        properties_key: &str,
    ) -> AppResult<()>;

    /// 列出连接（支持分页）
    pub async fn list_connections(
        &self,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ConnectionInfo>>;

    /// 列出信道（支持分页 + 按连接过滤）
    pub async fn list_channels(
        &self,
        connection_name: Option<&str>,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ChannelInfo>>;

    /// 队列列表支持分页
    pub async fn list_queues(
        &self,
        filter: &QueueFilter,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ManagementQueue>>;
}
```

### 3.2 限流器

```rust
// src-tauri/crates/core/src/rabbit/rate_limiter.rs

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

pub struct ManagementRateLimiter {
    permits: Arc<Semaphore>,
}

impl ManagementRateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        let permits = Arc::new(Semaphore::new(max_per_second as usize));
        let permits_clone = permits.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            loop {
                ticker.tick().await;
                permits_clone.add_permits(max_per_second as usize);
            }
        });
        Self { permits }
    }

    pub async fn acquire(&self) {
        let _ = self.permits.acquire().await;
    }
}
```

- 默认每秒 20 个 permit。
- 全局单例，通过 `lazy_static` 或 `OnceLock` 挂到 `ManagementClient`。
- 每个 HTTP 调用前 `acquire().await`。

### 3.3 降级与缓存

```rust
// src-tauri/crates/core/src/rabbit/management.rs

pub struct ManagementClient {
    client: Client,
    base_url: String,
    auth_header: String,
    vhost: String,
    rate_limiter: Arc<ManagementRateLimiter>,
}

/// 调用结果元信息
pub struct ApiCallOutcome<T> {
    pub data: T,
    pub is_stale: bool,
}

impl ManagementClient {
    async fn get_json_with_fallback<T: DeserializeOwned + Clone>(
        &self,
        path: &str,
        cache: &RwLock<Option<T>>,
    ) -> AppResult<ApiCallOutcome<T>> {
        self.rate_limiter.acquire().await;
        match self.get_json::<T>(path).await {
            Ok(data) => {
                *cache.write() = Some(data.clone());
                Ok(ApiCallOutcome { data, is_stale: false })
            }
            Err(e) => {
                if let Some(cached) = cache.read().clone() {
                    log::warn!("Management API 失败，返回缓存数据: {}", e);
                    Ok(ApiCallOutcome { data: cached, is_stale: true })
                } else {
                    Err(e)
                }
            }
        }
    }
}
```

- 仅对幂等读取接口启用缓存（overview、queues、connections、channels、consumers）。
- 写操作（Purge、delete_binding 等）不缓存。

## 四、自动刷新任务

### 4.1 任务生命周期

```rust
// src-tauri/crates/core/src/rabbit/refresh_task.rs

pub struct RefreshTask {
    handle: Mutex<Option<JoinHandle<()>>>,
    interval_ms: AtomicU64,
    enabled: AtomicBool,
}

impl RefreshTask {
    pub fn new() -> Self { ... }

    pub fn start(&self, app: AppHandle, state: Arc<AppState>) { ... }
    pub fn stop(&self) { ... }
    pub fn set_interval(&self, ms: u64) { ... }
    pub fn set_enabled(&self, enabled: bool) { ... }
}
```

- 绑定到 `AppState`，每次 `set_active` 时启动/重启任务。
- 任务内部循环：sleep → 拉取队列摘要 → 计算健康度 → `app.emit_to(window_label, "queue-refreshed", event)`。
- 窗口 label 通过 Tauri `Window` 获取，任务需要 `AppHandle`。

### 4.2 事件命名

- `queue-refreshed`：队列摘要更新事件（payload: `QueueRefreshEvent`）。
- `management-stale`：降级状态事件（payload: `{ is_stale: bool }`）。

## 五、Tauri 命令层

### 5.1 新增命令

```rust
// src-tauri/src/commands/binding.rs
#[tauri::command]
pub async fn list_queue_bindings(queue_name: String, state: State<'_, Arc<AppState>>) -> AppResult<Vec<BindingInfo>>;

#[tauri::command]
pub async fn delete_queue_binding(
    exchange: String,
    queue_name: String,
    properties_key: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()>;

// src-tauri/src/commands/queue.rs（扩展）
#[tauri::command]
pub async fn purge_queue(name: String, state: State<'_, Arc<AppState>>) -> AppResult<QueueSummary>;

// src-tauri/src/commands/connection.rs（扩展）
#[tauri::command]
pub async fn list_connections(
    pagination: Option<Pagination>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Paginated<ConnectionInfo>>;

// src-tauri/src/commands/channel.rs
#[tauri::command]
pub async fn list_channels(
    connection_name: Option<String>,
    pagination: Option<Pagination>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Paginated<ChannelInfo>>;

// src-tauri/src/commands/settings.rs（如需要）
#[tauri::command]
pub fn get_refresh_config(state: State<'_, Arc<AppState>>) -> RefreshConfig;

#[tauri::command]
pub fn set_refresh_config(config: RefreshConfig, state: State<'_, Arc<AppState>>) -> AppResult<()>;
```

### 5.2 命令注册

在 `src-tauri/src/lib.rs` 的 `generate_handler!` 宏中注册新增命令。

## 六、前端设计

### 6.1 新增/修改视图

- `src/views/ConnectionsView.jsx`：连接列表，点击行展开 Channel 面板。
- `src/views/ChannelsPanel.jsx`（或内嵌在 ConnectionsView）：信道列表。
- `src/views/QueueDetailView.jsx`：新增「绑定」Tab，新增「清空队列」按钮。
- `src/views/QueuesView.jsx`：接入虚拟滚动组件，新增分页控件。
- `src/views/SettingsView.jsx`：新增自动刷新开关与周期选择。

### 6.2 新增组件

- `src/components/VirtualTable.jsx`：基于固定行高 + transform 的虚拟滚动表格。
- `src/components/BindingRow.jsx`：绑定行展示与解绑按钮。
- `src/components/ConnectionRow.jsx` / `ChannelRow.jsx`。
- `src/components/RefreshToggle.jsx`：自动刷新开关 + 周期选择。

### 6.3 API 封装（src/lib/api.js）

```javascript
export async function purgeQueue(name) { return invoke("purge_queue", { name }); }
export async function listQueueBindings(queueName) { return invoke("list_queue_bindings", { queueName }); }
export async function deleteQueueBinding(exchange, queueName, propertiesKey) { ... }
export async function listConnections(pagination = {}) { return invoke("list_connections", { pagination }); }
export async function listChannels(connectionName, pagination = {}) { ... }
export async function listenQueueRefreshed(callback) { return listen("queue-refreshed", callback); }
export async function listenManagementStale(callback) { return listen("management-stale", callback); }
```

### 6.4

## 七、事件订阅与状态同步

### 7.1 前端事件监听

- 在 `OverviewView` 与 `QueuesView` 的 `useEffect` 中调用 `listenQueueRefreshed`。
- 收到事件后，若当前未处于手动编辑/弹窗状态，则直接更新 state，触发重渲染。
- `management-stale` 事件用于控制全局 banner 的显示/隐藏。

### 7.2 避免内存泄漏

- `listen` 返回的 unlisten 函数在 `useEffect` cleanup 中调用。
- 窗口切换/关闭时，Tauri 会自动停止对该窗口 label 的事件推送。

## 八、文件改动范围

### 8.1 允许修改的文件

| 文件/目录 | 改动内容 |
|-----------|----------|
| `src-tauri/crates/core/src/rabbit/management.rs` | 新增 Purge/Bindings/Connections/Channels 方法；增加分页参数；接入限流与缓存。 |
| `src-tauri/crates/core/src/rabbit/rate_limiter.rs` | 新增（可选单独文件）。 |
| `src-tauri/crates/core/src/rabbit/refresh_task.rs` | 新增后台刷新任务。 |
| `src-tauri/crates/core/src/rabbit/mod.rs` | 导出新增模块。 |
| `src-tauri/crates/core/src/models.rs` | 新增 `BindingInfo`、`ConnectionInfo`、`ChannelInfo`、`Pagination`、`Paginated`、`QueueRefreshEvent` 等模型。 |
| `src-tauri/crates/core/src/state.rs` | 接入 `RefreshTask`，存储刷新配置。 |
| `src-tauri/crates/core/src/error.rs` | 如需新增错误类型可扩展。 |
| `src-tauri/src/commands/queue.rs` | 新增 `purge_queue` 命令。 |
| `src-tauri/src/commands/binding.rs` | 新增 `list_queue_bindings`、`delete_queue_binding` 命令。 |
| `src-tauri/src/commands/connection.rs` | 扩展 `list_connections` 命令。 |
| `src-tauri/src/commands/channel.rs` | 新增 `list_channels` 命令。 |
| `src-tauri/src/commands/mod.rs` | 声明新增命令模块。 |
| `src-tauri/src/lib.rs` | 在 `generate_handler!` 中注册新命令。 |
| `src-tauri/Cargo.toml` | 如需新增依赖（如 `governor`）则添加。 |
| `src/lib/api.js` | 新增前端 API 与事件监听封装。 |
| `src/views/QueuesView.jsx` | 接入虚拟滚动与分页。 |
| `src/views/QueueDetailView.jsx` | 新增绑定 Tab 与清空按钮。 |
| `src/views/ConnectionsView.jsx` | 新增连接列表视图。 |
| `src/views/SettingsView.jsx` | 新增自动刷新开关。 |
| `src/components/VirtualTable.jsx` | 新增虚拟滚动表格组件。 |
| `src/components/BindingRow.jsx` | 新增绑定行组件。 |
| `src/components/ConnectionRow.jsx` | 新增连接行组件。 |
| `src/components/ChannelRow.jsx` | 新增信道行组件。 |
| `src/components/RefreshToggle.jsx` | 新增刷新开关组件。 |
| `src/app.jsx` | 注册 ConnectionsView 路由。 |
| `src/components/Sidebar.jsx` | 新增「连接」导航项。 |
| `src/styles/glass.css` | 补充表格/抽屉/ banner 样式（不修改核心 Token）。 |

### 8.2 禁止触碰的文件

| 文件/目录 | 原因 |
|-----------|------|
| `src/styles/tokens.css` | ADR 003：禁止修改核心 CSS 变量。 |
| `legacy/` | ADR 004：旧项目归档冻结。 |
| `src/main.jsx` | 无需改动，入口保持现状。 |
| `src-tauri/src/main.rs` | 除非新增插件，否则不改动入口。 |
| `src-tauri/crates/core/src/crypto.rs` | 加密逻辑本次不涉及。 |
| `src-tauri/crates/core/src/storage.rs` | 本次不新增持久化树（刷新配置可放内存或复用现有配置机制）。 |
| `src-tauri/crates/core/src/health.rs` | 健康度算法本次不变。 |
| `src-tauri/crates/core/src/rabbit/consumer_manager.rs` | 手动消费者本次不做改动。 |
| `src-tauri/crates/core/src/rabbit/publisher.rs` | AMQP 发布本次不做连接池改造。 |

## 九、依赖关系

### 9.1 新增 Rust 依赖（可选）

- `governor`：若不自研令牌桶，可用成熟的 Governor 限流库。
- 若自研则无需新增依赖（仅用 `tokio::sync::Semaphore`）。

### 9.2 前端依赖

- 不引入大型图表库或表格库；虚拟滚动自研。
- 如需事件监听，使用 `@tauri-apps/api/event` 的 `listen`。

## 十、风险与回滚

| 风险 | 缓解措施 |
|------|----------|
| 限流导致刷新延迟过大 | 默认 20 req/s，用户可配置；自动刷新事件单独走限流但不阻塞 UI。 |
| 后台任务泄漏 | `set_active` 切换时先 `stop` 再 `start`；窗口关闭时停止。 |
| 缓存数据误导用户 | 缓存仅用于读取接口，且必须展示 stale 标记；写操作不缓存。 |
| 分页破坏现有搜索过滤 | 搜索仍走服务端 + 客户端双重过滤；分页基于过滤后总数。 |

## 十一、验证策略

1. **单元测试**：`mqdesk-core` 中对 `ManagementClient` 新增方法、`RateLimiter`、`RefreshTask` 写测试。
2. **集成测试**：使用 mock HTTP server 验证 Management API 调用路径与 URL 编码。
3. **前端测试**：`VirtualTable` 渲染行数断言、`RefreshToggle` 状态变化测试。
4. **E2E / 人工**：在 500 队列的 RabbitMQ 容器上验证首屏加载、自动刷新、降级提示。
