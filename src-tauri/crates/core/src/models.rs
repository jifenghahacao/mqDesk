//! 数据模型：连接、队列、消息、追踪

use serde::{Deserialize, Serialize};

// === 连接 ===

/// 一个 RabbitMQ 连接配置（保存到本地）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub amqp_port: u16,
    pub management_port: u16,
    pub management_scheme: String,
    pub vhost: String,
    pub username: String,
    /// 密码不直接存这里，单独走 keyring 加密
    #[serde(skip_serializing, default)]
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建/更新连接的请求
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_amqp_port")]
    pub amqp_port: u16,
    #[serde(default = "default_management_port")]
    pub management_port: u16,
    #[serde(default = "default_management_scheme")]
    pub management_scheme: String,
    #[serde(default = "default_vhost")]
    pub vhost: String,
    pub username: String,
    pub password: String,
}

fn default_amqp_port() -> u16 {
    5672
}
fn default_management_port() -> u16 {
    15672
}
fn default_management_scheme() -> String {
    "http".to_string()
}
fn default_vhost() -> String {
    "/".to_string()
}

/// 连接实时状态（用于连接管理页展示每个连接是否活跃、是否可达）
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionStatus {
    pub id: String,
    pub name: String,
    pub host: String,
    pub management_port: u16,
    pub vhost: String,
    pub username: String,
    pub is_active: bool,
    pub is_reachable: bool,
    pub cluster_name: Option<String>,
    pub error: Option<String>,
}

/// RabbitMQ 集群节点信息
#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    pub name: String,
    pub is_running: bool,
    pub node_type: String,
    pub uptime_seconds: u64,
    pub mem_used_bytes: u64,
    pub mem_limit_bytes: u64,
    pub mem_usage_percent: f64,
    pub disk_free_bytes: u64,
    pub disk_free_limit_bytes: u64,
    /// 磁盘健康状态：ok / warn / danger
    pub disk_free_status: String,
    pub fd_used: u64,
    pub fd_total: u64,
    pub proc_used: u64,
    pub proc_total: u64,
    pub sockets_used: u64,
    pub sockets_total: u64,
}

/// RabbitMQ 消费者信息
#[derive(Debug, Clone, Serialize)]
pub struct ConsumerInfo {
    pub consumer_tag: String,
    pub queue_name: String,
    pub client_address: String,
    pub connection_name: String,
    pub ack_required: bool,
    pub prefetch_count: u32,
    /// 连接时长（秒）
    pub connected_seconds: u64,
    /// 消费速率（条/秒），取自队列 deliver_get 速率
    pub message_rate: f64,
}

// === 手动消费者（消费者工作室）===

/// 消息过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerFilter {
    #[serde(default = "default_filter_payload_type")]
    pub payload_type: String,
    #[serde(default)]
    pub payload_value: String,
    #[serde(default)]
    pub headers: Vec<HeaderFilter>,
}

fn default_filter_payload_type() -> String {
    "contains".to_string()
}

impl Default for ConsumerFilter {
    fn default() -> Self {
        Self {
            payload_type: "contains".to_string(),
            payload_value: String::new(),
            headers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderFilter {
    pub key: String,
    pub value: String,
}

/// 消费者配置（前端创建表单）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualConsumerConfig {
    pub name: String,
    pub queue_name: String,
    #[serde(default = "default_consumer_mode")]
    pub mode: String,
    #[serde(default = "default_prefetch_count")]
    pub prefetch_count: u16,
    /// true=自动确认（真实消费）；false=手动确认（默认预览模式，只看不消费）
    #[serde(default)]
    pub auto_ack: bool,
    #[serde(default)]
    pub filter: ConsumerFilter,
}

fn default_consumer_mode() -> String {
    "async".to_string()
}

fn default_prefetch_count() -> u16 {
    10
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
    /// pending / running / paused / destroyed / error
    pub status: String,
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
    pub headers: serde_json::Value,
    pub redelivered: bool,
    pub acked: bool,
}

// === 总览 ===

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub health: HealthStatus,
    pub summary: String,
    pub summary_detail: String,
    pub stats: OverviewStats,
    pub alerts: Vec<AlertItem>,
    pub recent_feed: Vec<MessageFeedItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewStats {
    pub queue_count: u64,
    pub exchange_count: u64,
    pub total_messages: u64,
    pub alert_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertItem {
    pub queue_name: String,
    pub health: HealthStatus,
    pub ready: u64,
    pub reason: String,
}

// === 健康度 ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Warn,
    Danger,
    Idle,
}

impl HealthStatus {
    pub fn label(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "正常",
            HealthStatus::Warn => "堆积预警",
            HealthStatus::Danger => "无人消费",
            HealthStatus::Idle => "空闲",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "ok",
            HealthStatus::Warn => "warn",
            HealthStatus::Danger => "danger",
            HealthStatus::Idle => "idle",
        }
    }
}

// === 队列 ===

#[derive(Debug, Clone, Serialize)]
pub struct QueueSummary {
    pub name: String,
    pub vhost: String,
    pub queue_type: String,
    pub durable: bool,
    pub auto_delete: bool,
    pub ready: u64,
    pub unacked: u64,
    pub total: u64,
    pub consumers: u64,
    pub health: HealthStatus,
    pub health_summary: String,
    pub incoming_rate: f64,
    pub outgoing_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueConnectionInfo {
    pub name: String,
    pub peer_address: String,
    pub connection_name: String,
    pub ack_required: Option<bool>,
    pub prefetch_count: Option<u32>,
    pub connected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueDetail {
    pub summary: QueueSummary,
    pub arguments: serde_json::Value,
    pub policy: Option<String>,
    pub rate_history: RateHistory,
    pub producers: Vec<QueueConnectionInfo>,
    pub consumers: Vec<QueueConnectionInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateHistory {
    pub incoming: Vec<f64>,
    pub outgoing: Vec<f64>,
    pub timestamps: Vec<String>,
}

// === 抓取预览 ===

#[derive(Debug, Clone, Serialize)]
pub struct PreviewMessage {
    pub routing_key: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub payload_preview: String,
    pub headers: serde_json::Value,
}

// === 队列消息（peek，不消费）===

#[derive(Debug, Clone, Serialize)]
pub struct QueueMessage {
    pub delivery_tag: u64,
    pub exchange: String,
    pub routing_key: String,
    pub payload: String,
    pub payload_size: u64,
    pub headers: serde_json::Value,
    pub properties: serde_json::Value,
    pub redelivered: bool,
}

// === 队列筛选 ===

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueueFilter {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub queue_type: String,
    #[serde(default)]
    pub health: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateQueueInput {
    pub name: String,
    pub vhost: String,
    #[serde(default = "default_queue_type")]
    pub queue_type: String,
    #[serde(default)]
    pub durable: bool,
    #[serde(default)]
    pub auto_delete: bool,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

fn default_queue_type() -> String {
    "classic".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueuePolicyInput {
    pub name: String,
    pub vhost: String,
    #[serde(default)]
    pub max_length: Option<i64>,
    #[serde(default)]
    pub message_ttl: Option<i64>,
    #[serde(default)]
    pub dead_letter_exchange: Option<String>,
    #[serde(default)]
    pub dead_letter_routing_key: Option<String>,
}

// === 队列告警 ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAlertRule {
    pub queue_name: String,
    pub vhost: String,
    pub metric: String, // ready_count | consumer_count | consumer_latency | incoming_rate
    pub operator: String, // gt | eq | lt
    pub threshold: f64,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAlertRecord {
    pub id: String,
    pub queue_name: String,
    pub vhost: String,
    pub metric: String,
    pub threshold: f64,
    pub actual_value: f64,
    pub triggered_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
}

// === 审计日志 ===

#[derive(Debug, Clone, Deserialize)]
pub struct AuditFilter {
    pub queue_name: Option<String>,
    pub vhost: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAuditLog {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub target_queue: String,
    pub vhost: String,
    pub detail: String,
    pub operator: String,
}

// === 发送消息 ===

#[derive(Debug, Clone, Deserialize)]
pub struct PublishRequest {
    /// 目标队列（直发模式）
    pub target_queue: Option<String>,
    /// 交换机（经交换机模式）
    pub exchange: Option<String>,
    pub routing_key: String,
    pub payload: String,
    pub content_type: String,
    #[serde(default)]
    pub headers: serde_json::Value,
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: u8,
    #[serde(default)]
    pub mandatory: bool,
}

fn default_delivery_mode() -> u8 {
    2 // persistent
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishResult {
    pub trace_id: String,
    pub status: PublishStatus,
    pub reply_code: i32,
    pub reply_text: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublishStatus {
    Confirmed,
    Returned,
    Failed,
}

// === 消息通知列表 ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFeedItem {
    pub trace_id: String,
    pub time: String,
    pub direction: MessageDirection,
    pub queue_name: String,
    pub exchange: Option<String>,
    pub routing_key: String,
    pub status: MessageStatus,
    pub summary: String,
    pub payload_preview: String,
    pub payload_size: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Sent,
    Received,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Sent,
    Consumed,
    Backlog,
    Failed,
}

impl MessageStatus {
    pub fn label(&self) -> &'static str {
        match self {
            MessageStatus::Sent => "已发送",
            MessageStatus::Consumed => "已被消费",
            MessageStatus::Backlog => "仍堆积",
            MessageStatus::Failed => "消费失败",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            MessageStatus::Sent => "s-sent",
            MessageStatus::Consumed => "s-consumed",
            MessageStatus::Backlog => "s-backlog",
            MessageStatus::Failed => "s-fail",
        }
    }

    /// 与前端筛选值一致的小写状态名
    pub fn value(&self) -> &'static str {
        match self {
            MessageStatus::Sent => "sent",
            MessageStatus::Consumed => "consumed",
            MessageStatus::Backlog => "backlog",
            MessageStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedFilter {
    pub queue: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

// === 绑定 / 连接 / 信道 / 分页 / 刷新事件 ===

/// 队列绑定信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingInfo {
    /// 交换机名，空字符串表示默认交换机直接绑定
    pub source: String,
    pub vhost: String,
    /// 目标队列名
    pub destination: String,
    pub destination_type: String,
    pub routing_key: String,
    pub arguments: serde_json::Value,
    /// 用于删除绑定的 key
    pub properties_key: String,
}

/// RabbitMQ 连接摘要
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub peer_host: String,
    pub peer_port: u16,
    /// 聚合字段：host:port
    pub peer_address: String,
    pub protocol: String,
    /// 连接时间戳（毫秒）
    pub connected_at: u64,
    /// 已连接时长（秒）
    pub connected_seconds: u64,
    pub channel_count: u32,
    /// running / blocked / blocking
    pub state: String,
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

/// 分页参数
#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    50
}

/// 分页结果包装
#[derive(Debug, Clone, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// 自动刷新事件 payload
#[derive(Debug, Clone, Serialize)]
pub struct QueueRefreshEvent {
    pub queues: Vec<QueueSummary>,
    pub overall_health: HealthStatus,
    pub alert_count: u64,
    pub is_stale: bool,
}
