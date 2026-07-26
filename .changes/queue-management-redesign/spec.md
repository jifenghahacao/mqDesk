# 队列管理页重构技术规格

## 一、数据模型

### 1.1 QueueSummary（列表项）

扩展现有 `QueueSummary`：

```rust
pub struct QueueSummary {
    pub name: String,
    pub vhost: String,
    pub queue_type: String,        // classic / quorum / stream
    pub durable: bool,
    pub auto_delete: bool,
    pub ready: u64,
    pub unacked: u64,
    pub total: u64,
    pub consumers: u64,
    pub incoming_rate: f64,
    pub outgoing_rate: f64,
    pub health: HealthStatus,
}
```

### 1.2 QueueDetail（详情）

新增：

```rust
pub struct QueueDetail {
    pub summary: QueueSummary,
    pub arguments: serde_json::Value,   // x-max-length, x-message-ttl, x-dead-letter-exchange 等
    pub policy: Option<String>,
    pub rate_history: RateHistory,      // 按小时/天/周聚合
    pub producers: Vec<QueueConnectionInfo>,
    pub consumers: Vec<QueueConnectionInfo>,
}

pub struct QueueConnectionInfo {
    pub name: String,
    pub peer_address: String,
    pub channel_details: Option<serde_json::Value>,
    pub connected_at: Option<String>,
}

pub struct RateHistory {
    pub incoming: Vec<f64>,
    pub outgoing: Vec<f64>,
    pub timestamps: Vec<String>,
}
```

### 1.3 QueueMessage（消息）

新增：

```rust
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
```

### 1.4 QueueAlertRule / QueueAlert

新增：

```rust
pub struct QueueAlertRule {
    pub queue_name: String,
    pub vhost: String,
    pub metric: String,      // ready_count / consumer_count / consumer_latency / incoming_rate
    pub operator: String,    // gt / eq / lt
    pub threshold: f64,
    pub enabled: bool,
}

pub struct QueueAlert {
    pub id: String,
    pub queue_name: String,
    pub vhost: String,
    pub metric: String,
    pub threshold: f64,
    pub actual_value: f64,
    pub triggered_at: String,
    pub resolved_at: Option<String>,
}
```

### 1.5 QueueAuditLog

新增：

```rust
pub struct QueueAuditLog {
    pub id: String,
    pub timestamp: String,
    pub action: String,      // create / update / delete / pause / resume / republish / move / delete_message
    pub target_queue: String,
    pub vhost: String,
    pub detail: String,
    pub operator: String,    // 当前系统用户名
}
```

## 二、后端命令（Tauri）

### 2.1 队列查询

- `list_queues(filter: QueueFilter) -> Vec<QueueSummary>`
- `get_queue_detail(name: String, vhost: String) -> QueueDetail`

### 2.2 队列操作

- `create_queue(config: CreateQueueInput) -> QueueSummary`
- `update_queue_policy(name: String, vhost: String, policy: QueuePolicyInput) -> QueueSummary`
- `delete_queue(name: String, vhost: String, backup_count: Option<u32>) -> ()`
- `pause_queue(name: String, vhost: String) -> QueueSummary`
- `resume_queue(name: String, vhost: String) -> QueueSummary`

### 2.3 消息管理

- `peek_queue_messages(name: String, vhost: String, count: u32) -> Vec<QueueMessage>`
- `republish_message(name: String, vhost: String, delivery_tag: u64) -> ()`
- `move_message(source: String, target: String, vhost: String, delivery_tag: u64, keep_original: bool) -> ()`
- `delete_message(name: String, vhost: String, delivery_tag: u64) -> ()`

### 2.4 告警

- `list_queue_alert_rules() -> Vec<QueueAlertRule>`
- `set_queue_alert_rule(rule: QueueAlertRule) -> QueueAlertRule`
- `delete_queue_alert_rule(queue_name: String, vhost: String, metric: String) -> ()`
- `list_queue_alerts(resolved: Option<bool>) -> Vec<QueueAlert>`

### 2.5 审计

- `list_queue_audit_logs(filter: AuditFilter) -> Vec<QueueAuditLog>`
- `export_queue_audit_logs(filter: AuditFilter, path: String) -> ()`

### 2.6 性能

- `get_queue_performance_report(name: String, vhost: String) -> QueuePerformanceReport`

## 三、管理 API 数据来源

- 队列列表：`GET /api/queues`
- 队列详情：`GET /api/queues/{vhost}/{name}`
- 队列声明：`PUT /api/queues/{vhost}/{name}`
- 删除队列：`DELETE /api/queues/{vhost}/{name}`
- 暂停/恢复：`PUT /api/queues/{vhost}/{name}`（设置 `paused` 字段）
- Peek 消息：`POST /api/queues/{vhost}/{name}/messages/get`
- 删除单条：`DELETE /api/queues/{vhost}/{name}/messages`（通过 delivery_tag）

## 四、前端页面

### 4.1 新建/修改文件

- `src/views/QueuesView.jsx`：重构列表页。
- `src/views/QueueDetailDrawer.jsx`：新增详情抽屉。
- `src/views/QueueMessageModal.jsx`：新增消息详情弹窗。
- `src/views/QueueFormModal.jsx`：新增新建/编辑队列弹窗。
- `src/components/QueueAlertPanel.jsx`：告警规则/历史。
- `src/components/QueueAuditPanel.jsx`：审计日志。
- `src/components/QueuePerformancePanel.jsx`：性能诊断。
- `src/components/SimpleLineChart.jsx`：SVG 趋势图。
- `src/lib/api.js`：新增队列相关 API。
- `src/styles/glass.css`：补充表格、抽屉、弹窗样式。

### 4.2 路由

不需要新增路由，队列管理仍使用现有 `queues` 视图；详情通过抽屉展示。

## 五、告警检查机制

- 后台任务每 30 秒拉取一次队列列表。
- 对每个启用规则的队列计算指标，触发阈值则写入告警表并发送 Toast。
- 指标恢复后标记 resolved_at。

## 六、审计日志存储

- 使用现有 `Storage`（sled）新增 `queue_audit_logs` tree。
- 保留最近 1000 条，超出后按时间删除最旧记录。

## 七、性能报告算法

- **堆积原因**：
  - consumers == 0 → 无人消费
  - incoming_rate > outgoing_rate * 2 → 生产过快
  - outgoing_rate == 0 → 消费停滞
- **建议**：
  - classic + 高可用需求 → 建议迁移到 quorum
  - 消息 TTL 未设置 + 堆积 → 建议设置 TTL 或死信策略

## 八、安全与确认

- 删除队列、删除消息、移动消息需要二次确认。
- 危险操作写入审计日志。
- 不实现 RBAC，桌面端默认当前系统用户为操作人。
