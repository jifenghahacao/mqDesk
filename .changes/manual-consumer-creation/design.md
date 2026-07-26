# Design：MQDesk 手动消费者创建（Consumer Studio）

## 1. 架构总览

```
前端 (Preact)
  ├─ views/ConsumerStudioView.jsx     新增：消费者工作室主页面
  ├─ components/ConsumerForm.jsx      新增：消费者配置表单
  ├─ components/ConsumerList.jsx      新增：消费者卡片列表
  ├─ components/ConsumerMessageList.jsx 新增：已消费消息列表
  ├─ components/Sidebar.jsx           改：添加「消费者工作室」导航入口
  ├─ app.jsx                          改：注册 consumer-studio 路由
  └─ lib/api.js                       改：封装消费者相关命令

Tauri 命令层 (src-tauri/src/commands/)
  ├─ consumer.rs                      改/新增：create/start/pause/resume/destroy/list_messages/get_status
  └─ lib.rs                           改：注册命令

crates/core
  ├─ src/models.rs                    改：新增 ManualConsumer、ConsumerMessage、ConsumerFilter 等模型
  ├─ src/rabbit/consumer_manager.rs   新增：AMQP 消费者生命周期管理器
  ├─ src/rabbit/mod.rs                改：导出 consumer_manager
  └─ src/state.rs                     改：AppState 增加 ConsumerManager
```

## 2. 数据模型

### 2.1 Rust 模型（`src-tauri/crates/core/src/models.rs`）

```rust
/// 消息过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerFilter {
    pub payload_type: String, // "contains" | "equals" | "regex"
    pub payload_value: String,
    pub headers: Vec<HeaderFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderFilter {
    pub key: String,
    pub value: String,
}

/// 消费者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualConsumerConfig {
    pub name: String,
    pub queue_name: String,
    pub mode: String,        // "sync" | "async"
    pub prefetch_count: u16, // 仅在 async 时有效，默认 10
    pub auto_ack: bool,      // true=自动确认（真实消费），false=手动确认（默认预览模式，只看不消费）
    pub filter: ConsumerFilter,
}

/// 消费者运行时状态
#[derive(Debug, Clone, Serialize)]
pub struct ManualConsumer {
    pub id: String,
    pub name: String,
    pub queue_name: String,
    pub mode: String,
    pub prefetch_count: u16,
    pub auto_ack: bool,
    pub status: String, // "pending" | "running" | "paused" | "destroyed" | "error"
    pub error: Option<String>,
    pub consumed_count: u64,
    pub filtered_count: u64,
}

/// 已消费消息
#[derive(Debug, Clone, Serialize)]
pub struct ConsumerMessage {
    pub id: String,
    pub consumer_id: String,
    pub timestamp_ms: u64,
    pub exchange: String,
    pub routing_key: String,
    pub payload: String,
    pub payload_size: u64,
    pub headers: HashMap<String, String>,
    pub redelivered: bool,
    pub acked: bool,
}
```

### 2.2 Tauri 命令

| 命令 | 输入 | 输出 | 说明 |
|---|---|---|---|
| `create_consumer` | `config: ManualConsumerConfig` | `ManualConsumer` | 创建但不启动 |
| `start_consumer` | `id: String` | `ManualConsumer` | 开始消费 |
| `pause_consumer` | `id: String` | `ManualConsumer` | 暂停消费 |
| `resume_consumer` | `id: String` | `ManualConsumer` | 继续消费 |
| `destroy_consumer` | `id: String` | `()` | 销毁并清理 |
| `list_consumers` | - | `Vec<ManualConsumer>` | 列出所有消费者状态 |
| `list_consumer_messages` | `id: String, limit: u32` | `Vec<ConsumerMessage>` | 获取消息列表 |
| `ack_message` | `consumer_id: String, message_id: String` | `()` | 手动确认消息 |
| `clear_consumer_messages` | `id: String` | `()` | 清空前端列表 |

## 3. 消费者管理器（`consumer_manager.rs`）

### 3.1 职责
- 维护当前会话中所有手动消费者的运行时状态
- 使用 lapin 库建立 AMQP 消费者连接
- 在消息到达时应用过滤条件
- 将过滤后的消息存入内存环形缓冲区（默认保留最近 500 条）
- 处理生命周期：pending → running → paused → running → destroyed

### 3.2 过滤逻辑
过滤在 Rust 端执行，确保前端只收到匹配消息：

```rust
fn matches_filter(msg: &ConsumerMessage, filter: &ConsumerFilter) -> bool {
    // payload 过滤
    let payload_ok = match filter.payload_type.as_str() {
        "contains" => msg.payload.contains(&filter.payload_value),
        "equals" => msg.payload == filter.payload_value,
        "regex" => Regex::new(&filter.payload_value).map(|re| re.is_match(&msg.payload)).unwrap_or(false),
        _ => true,
    };

    // headers 过滤
    let headers_ok = filter.headers.iter().all(|h| {
        msg.headers.get(&h.key).map(|v| v == &h.value).unwrap_or(false)
    });

    payload_ok && headers_ok
}
```

### 3.3 Ack 模式
- **手动 Ack（默认预览模式）**：消息到达后只进入消息列表，不自动确认；用户可逐条点击「确认」真正消费，或切换为真实消费模式后自动确认；关闭/销毁消费者时未确认消息重新入队（nack with requeue=true），保证只看不消费。
- **自动 Ack（真实消费模式）**：lapin 消息使用 `BasicAckOptions::default()` 自动确认，消息会从队列移除。

### 3.4 并发模式
- **同步**：`basic_consume` 默认逐条回调，prefetch=1。
- **异步**：设置 `basic_qos` prefetch_count，回调中并发处理但过滤和存储加锁。

## 4. 前端设计

### 4.1 页面布局

```
┌─────────────────────────────────────────────────────────────┐
│  消费测试                                          [刷新]    │
├──────────────────────┬──────────────────────────────────────┤
│                      │                                      │
│  消费者配置表单      │   消费者卡片列表                       │
│  ─────────────       │   ┌─ 消费者 A ─┐ ┌─ 消费者 B ─┐       │
│  名称 [        ]     │   │ 运行中      │ │ 已暂停      │       │
│  队列 [▼       ]     │   │ 队列: q1   │ │ 队列: q2   │       │
│  模式 ○同步 ●异步    │   │ [暂停][销毁]│ │ [继续][销毁]│       │
│  预取 [    10]       │   └───────────┘ └───────────┘       │
│  Ack  ○自动 ●手动    │                                      │
│  过滤条件            │   已消费消息                          │
│  ┌────────────────┐  │   # 时间   路由键   Payload  状态    │
│  │ payload 包含   │  │   1  ...   ...      ...      已确认  │
│  │ headers        │  │   2  ...   ...      ...      待确认  │
│  └────────────────┘  │                                      │
│  [创建消费者]        │   [清空列表]                          │
│                      │                                      │
└──────────────────────┴──────────────────────────────────────┘
```

### 4.2 组件拆分

- `ConsumerStudioView`：页面容器，管理消费者列表和消息列表状态，定时拉取状态。
- `ConsumerForm`：表单，包含名称、队列、模式、预取、Ack、过滤条件。
- `ConsumerList`：消费者卡片网格，每个卡片展示状态和控制按钮。
- `ConsumerMessageList`：消息表格，支持展开详情、手动 ack、清空。

### 4.3 实时更新
- 前端通过 `setInterval` 每 2 秒调用 `list_consumers` 和 `list_consumer_messages` 刷新状态。
- 消息列表按 `timestamp_ms` 倒序排列。

## 5. 依赖关系

- `consumer_manager` 依赖 `AppState::active_connection` 获取当前 AMQP 连接参数。
- `create_consumer` 依赖 `list_queues` 能力验证队列存在。
- 前端 `ConsumerStudioView` 依赖当前活跃连接存在，否则展示空状态。
- 不依赖新增外部 crate，复用现有 `lapin`（已用于 AMQP 连接）。

## 6. 禁止修改的文件

| 文件/目录 | 原因 |
|---|---|
| `legacy/` | AGENTS.md 禁止项 |
| `src/styles/tokens.css` | 液态玻璃核心变量禁止修改 |
| `src-tauri/crates/core/src/crypto.rs` | 加密方案已稳定 |
| `src-tauri/crates/core/src/storage.rs` 数据结构 | 不引入新持久化表 |

## 7. 测试策略

| 层级 | 方式 |
|---|---|
| Rust core | `cargo test`：mock consumer_manager 状态转换，验证过滤函数 |
| Tauri 命令 | `cargo test`：调用 command handler 验证参数校验 |
| 前端组件 | Vitest + @testing-library/preact：mock `api.js` 返回值 |
| 全量门禁 | `python tooling/checks.py guard` |

## 8. 风险与回退

- **AMQP 连接复用风险**：消费者应复用现有 AMQP 连接，避免每次创建新 TCP 连接。若复用复杂，可临时创建独立连接，但需在文档中说明。
- **消息过滤正则不合法**：前端和 Rust 均需校验，避免运行时 panic。
- **手动 Ack 未确认消息**：消费者销毁时若未 nack，消息会保留未确认状态。必须保证 nack with requeue=true。
- **内存溢出**：消息缓冲区限制 500 条，超出时丢弃最旧消息。
