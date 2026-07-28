// 浏览器截图专用 Mock API
// 仅在本地开发/截图时通过 src/lib/api.js 临时引入，不会进入生产构建。

const NO_CONNECTION_MODE = typeof window !== "undefined" && new URLSearchParams(window.location.search).has("noConnection");

const MOCK_CONNECTION = {
  id: "conn-1",
  name: "本地开发环境",
  host: "127.0.0.1",
  amqp_port: 5672,
  management_port: 15672,
  management_scheme: "http",
  vhost: "/",
  username: "guest",
  password: "******",
};

const QUEUES = [
  { name: "order.created", vhost: "/", queue_type: "classic", ready: 1240, unacked: 12, total: 1252, consumers: 2, incoming_rate: 45.2, outgoing_rate: 38.6, health: "warn" },
  { name: "payment.callback", vhost: "/", queue_type: "classic", ready: 0, unacked: 0, total: 0, consumers: 0, incoming_rate: 0, outgoing_rate: 0, health: "idle" },
  { name: "notify.sms", vhost: "/", queue_type: "quorum", ready: 86, unacked: 4, total: 90, consumers: 3, incoming_rate: 12.5, outgoing_rate: 12.1, health: "ok" },
  { name: "notify.email", vhost: "/", queue_type: "classic", ready: 3200, unacked: 0, total: 3200, consumers: 0, incoming_rate: 28.0, outgoing_rate: 0, health: "danger" },
  { name: "log.audit", vhost: "/", queue_type: "stream", ready: 560, unacked: 0, total: 560, consumers: 1, incoming_rate: 8.4, outgoing_rate: 8.4, health: "ok" },
  { name: "user.signup", vhost: "/", queue_type: "classic", ready: 12, unacked: 0, total: 12, consumers: 1, incoming_rate: 2.1, outgoing_rate: 2.1, health: "ok" },
];

const OVERVIEW = {
  health: "warn",
  summary: "系统存在 1 个告警",
  summary_detail: "order.created 队列消息堆积超过 1000 条，notify.email 队列暂无消费者。",
  stats: {
    queue_count: 6,
    exchange_count: 14,
    total_messages: 5114,
    alert_count: 2,
  },
  alerts: [
    { queue_name: "order.created", health: "warn", reason: "待消费消息数 1240，超过阈值 1000", ready: 1240 },
    { queue_name: "notify.email", health: "danger", reason: "队列有消息但无消费者", ready: 3200 },
  ],
  recent_feed: [
    { trace_id: "t-1001", time: "14:32:05", queue_name: "order.created", status: "backlog", summary: "订单创建消息", direction: "sent" },
    { trace_id: "t-1002", time: "14:31:48", queue_name: "notify.sms", status: "consumed", summary: "短信验证码", direction: "sent" },
    { trace_id: "t-1003", time: "14:31:22", queue_name: "log.audit", status: "consumed", summary: "审计日志", direction: "sent" },
    { trace_id: "t-1004", time: "14:30:55", queue_name: "notify.email", status: "backlog", summary: "邮件通知", direction: "sent" },
  ],
};

const NODES = [
  {
    name: "rabbit@mqdesk-node1",
    is_running: true,
    node_type: "disc",
    uptime_seconds: 86400 + 3600 * 6,
    mem_used_bytes: 1024 * 1024 * 312,
    mem_limit_bytes: 1024 * 1024 * 1024,
    mem_usage_percent: 30.5,
    disk_free_bytes: 1024 * 1024 * 1024 * 42,
    disk_free_limit_bytes: 1024 * 1024 * 1024 * 2,
    disk_free_status: "ok",
    fd_used: 48,
    fd_total: 1024,
    proc_used: 256,
    proc_total: 1048576,
    sockets_used: 12,
    sockets_total: 829,
  },
  {
    name: "rabbit@mqdesk-node2",
    is_running: true,
    node_type: "disc",
    uptime_seconds: 86400 + 1800,
    mem_used_bytes: 1024 * 1024 * 288,
    mem_limit_bytes: 1024 * 1024 * 1024,
    mem_usage_percent: 28.1,
    disk_free_bytes: 1024 * 1024 * 1024 * 38,
    disk_free_limit_bytes: 1024 * 1024 * 1024 * 2,
    disk_free_status: "ok",
    fd_used: 42,
    fd_total: 1024,
    proc_used: 240,
    proc_total: 1048576,
    sockets_used: 10,
    sockets_total: 829,
  },
];

const CONSUMERS = [
  { consumer_tag: "amq.ctag-abc", queue_name: "order.created", client_address: "192.168.1.12:54321", connected_seconds: 3600 * 2 + 120, message_rate: 18.3, ack_required: true, prefetch_count: 10, connection_name: "order-service-1" },
  { consumer_tag: "amq.ctag-def", queue_name: "order.created", client_address: "192.168.1.13:43210", connected_seconds: 3600 * 1 + 540, message_rate: 20.3, ack_required: true, prefetch_count: 10, connection_name: "order-service-2" },
  { consumer_tag: "amq.ctag-ghi", queue_name: "notify.sms", client_address: "192.168.1.20:33102", connected_seconds: 7200, message_rate: 4.0, ack_required: true, prefetch_count: 5, connection_name: "notify-service" },
  { consumer_tag: "amq.ctag-jkl", queue_name: "notify.sms", client_address: "192.168.1.21:44123", connected_seconds: 6900, message_rate: 4.1, ack_required: true, prefetch_count: 5, connection_name: "notify-service" },
  { consumer_tag: "amq.ctag-mno", queue_name: "log.audit", client_address: "192.168.1.30:51234", connected_seconds: 1800, message_rate: 8.4, ack_required: false, prefetch_count: 1, connection_name: "audit-service" },
];

const MESSAGE_FEED = [
  { trace_id: "t-1001", time: "14:32:05", queue_name: "order.created", status: "backlog", summary: "订单 20931 创建", direction: "sent", exchange: "", routing_key: "order.created", content_type: "application/json", payload_size: 156, payload_preview: '{"orderId":20931,"amount":9900}' },
  { trace_id: "t-1002", time: "14:31:48", queue_name: "notify.sms", status: "consumed", summary: "短信验证码 892341", direction: "sent", exchange: "notify.ex", routing_key: "sms", content_type: "application/json", payload_size: 88, payload_preview: '{"phone":"138****1234","code":"892341"}' },
  { trace_id: "t-1003", time: "14:31:22", queue_name: "log.audit", status: "consumed", summary: "用户登录日志", direction: "sent", exchange: "audit.ex", routing_key: "login", content_type: "application/json", payload_size: 132, payload_preview: '{"userId":1001,"action":"login"}' },
  { trace_id: "t-1004", time: "14:30:55", queue_name: "notify.email", status: "backlog", summary: "订单发货通知", direction: "sent", exchange: "notify.ex", routing_key: "email", content_type: "application/json", payload_size: 210, payload_preview: '{"orderId":20930,"subject":"shipped"}' },
];

const MANUAL_CONSUMERS = [
  { id: "mc-1", name: "测试-订单消费者", queue_name: "order.created", mode: "async", prefetch_count: 10, auto_ack: false, status: "running", created_at: Date.now() - 3600000, consumed_count: 12, filtered_count: 0, ack_count: 12 },
  { id: "mc-2", name: "测试-短信预览", queue_name: "notify.sms", mode: "sync", prefetch_count: 1, auto_ack: true, status: "paused", created_at: Date.now() - 7200000, consumed_count: 45, filtered_count: 2, ack_count: 40 },
];

const BINDINGS = {
  "order.created": [
    { source: "order.exchange", routing_key: "order.created", arguments: {} },
    { source: "dead.letter.exchange", routing_key: "order.created.dlx", arguments: { "x-dead-letter-exchange": "dead.letter.exchange" } },
  ],
  "notify.sms": [{ source: "notify.exchange", routing_key: "sms", arguments: {} }],
  "notify.email": [{ source: "notify.exchange", routing_key: "email", arguments: {} }],
};

const RABBIT_CONNECTIONS = [
  { name: "192.168.1.12:49321", peer_address: "192.168.1.12:49321", peer_host: "192.168.1.12", protocol: "AMQP 0-9-1", channels: 2, consumers: 4, connected_seconds: 8640 },
  { name: "192.168.1.15:51208", peer_address: "192.168.1.15:51208", peer_host: "192.168.1.15", protocol: "AMQP 0-9-1", channels: 1, consumers: 1, connected_seconds: 2700 },
  { name: "127.0.0.1:54123", peer_address: "127.0.0.1:54123", peer_host: "127.0.0.1", protocol: "Management", channels: 0, consumers: 0, connected_seconds: 720 },
];

const CHANNELS = {
  "192.168.1.12:49321": [
    { number: 1, connection_name: "192.168.1.12:49321", consumers: 2, prefetch_count: 10, publish_rate: 0, deliver_rate: 45, ack_rate: 45 },
    { number: 2, connection_name: "192.168.1.12:49321", consumers: 0, prefetch_count: 0, publish_rate: 12, deliver_rate: 0, ack_rate: 0 },
  ],
  "192.168.1.15:51208": [
    { number: 1, connection_name: "192.168.1.15:51208", consumers: 1, prefetch_count: 5, publish_rate: 0, deliver_rate: 8, ack_rate: 8 },
  ],
};

const ALERT_RULES = [
  { metric: "ready_count", operator: "gt", threshold: 1000, enabled: true },
  { metric: "consumer_count", operator: "lt", threshold: 1, enabled: true },
];

const ALERT_RECORDS = [
  { id: "ar-1", triggered_at: Date.now() - 300000, metric: "ready_count", threshold: 1000, actual_value: 1240, resolved_at: null },
  { id: "ar-2", triggered_at: Date.now() - 600000, metric: "consumer_count", threshold: 1, actual_value: 0, resolved_at: null },
];

const AUDIT_LOGS = [
  { id: "al-1", timestamp: Date.now() - 1800000, action: "create_queue", operator: "admin", detail: "创建队列 order.created" },
  { id: "al-2", timestamp: Date.now() - 3600000, action: "update_queue_policy", operator: "admin", detail: "更新 notify.sms 策略" },
  { id: "al-3", timestamp: Date.now() - 7200000, action: "pause_queue", operator: "admin", detail: "暂停队列 payment.callback" },
];

function delay(ms = 200) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function listConnections() {
  await delay();
  return NO_CONNECTION_MODE ? [] : [MOCK_CONNECTION];
}

export async function getConnection(id) {
  await delay();
  return MOCK_CONNECTION;
}

export async function createConnection(input) {
  await delay();
  return { ...input, id: `conn-${Date.now()}` };
}

export async function updateConnection(id, input) {
  await delay();
  return { ...input, id };
}

export async function deleteConnection(id) {
  await delay();
  return true;
}

export async function testConnection(input) {
  await delay(400);
  return "连接测试成功";
}

export async function connectTo(id) {
  await delay(300);
  return MOCK_CONNECTION;
}

export async function disconnect() {
  await delay();
  return true;
}

export async function getActiveConnection() {
  await delay();
  return NO_CONNECTION_MODE ? null : MOCK_CONNECTION;
}

export async function getConnectionStatus(id) {
  await delay();
  return {
    id,
    name: MOCK_CONNECTION.name,
    host: MOCK_CONNECTION.host,
    management_port: MOCK_CONNECTION.management_port,
    vhost: MOCK_CONNECTION.vhost,
    username: MOCK_CONNECTION.username,
    is_active: true,
    is_reachable: true,
    cluster_name: "mqdesk-cluster",
  };
}

export async function restoreLastActive() {
  await delay();
  return NO_CONNECTION_MODE ? null : MOCK_CONNECTION;
}

export async function listNodes() {
  await delay();
  return NODES;
}

export async function listConsumers() {
  await delay();
  return CONSUMERS;
}

export async function createManualConsumer(config) {
  await delay();
  return { id: `mc-${Date.now()}`, ...config, status: "running", created_at: Date.now(), consumed_count: 0, ack_count: 0 };
}

export async function startManualConsumer(id) {
  await delay();
  return true;
}

export async function pauseManualConsumer(id) {
  await delay();
  return true;
}

export async function resumeManualConsumer(id) {
  await delay();
  return true;
}

export async function destroyManualConsumer(id) {
  await delay();
  return true;
}

export async function getManualConsumer(id) {
  await delay();
  return MANUAL_CONSUMERS.find((c) => c.id === id) || MANUAL_CONSUMERS[0];
}

export async function listManualConsumers() {
  await delay();
  return MANUAL_CONSUMERS;
}

export async function listManualConsumerMessages(id, limit = 50) {
  await delay();
  return [
    { id: "m-1", delivery_tag: 101, payload_preview: '{"orderId":20931}', received_at: Date.now() - 120000, acked: true },
    { id: "m-2", delivery_tag: 102, payload_preview: '{"orderId":20932}', received_at: Date.now() - 60000, acked: true },
  ].slice(0, limit);
}

export async function ackManualConsumerMessage(consumerId, messageId) {
  await delay();
  return true;
}

export async function getOverview() {
  await delay(300);
  return OVERVIEW;
}

export async function listQueues(filter = {}) {
  await delay(300);
  let list = [...QUEUES];
  if (filter.search) {
    const kw = filter.search.toLowerCase();
    list = list.filter((q) => q.name.toLowerCase().includes(kw));
  }
  if (filter.queue_type && filter.queue_type !== "all") {
    list = list.filter((q) => q.queue_type === filter.queue_type);
  }
  if (filter.health && filter.health !== "all") {
    list = list.filter((q) => q.health === filter.health);
  }
  return list;
}

export async function getQueueDetail(name) {
  await delay(300);
  const q = QUEUES.find((x) => x.name === name) || QUEUES[0];
  const incoming = [12, 18, 22, 30, 45, 38, 42, 50, 48, 55, 60, 45];
  const outgoing = [10, 15, 20, 25, 30, 32, 35, 40, 42, 45, 48, 38];
  return {
    summary: { ...q, health_summary: q.health === "warn" ? "待消费消息超过阈值" : q.health === "danger" ? "队列有消息但无消费者" : "队列运行正常" },
    rate_history: { incoming, outgoing },
    arguments: { "x-queue-type": q.queue_type, durable: true },
    policy: q.name === "notify.sms" ? "ha-all" : null,
    consumers: CONSUMERS.filter((c) => c.queue_name === q.name),
  };
}

export async function grabPreview(queueName, count = 10) {
  await delay(300);
  return peekQueueMessages(queueName, count);
}

export async function peekQueueMessages(queueName, count = 20) {
  await delay(300);
  return Array.from({ length: Math.min(count, 5) }).map((_, i) => ({
    delivery_tag: 1001 + i,
    exchange: i % 2 === 0 ? "" : "notify.ex",
    routing_key: queueName,
    payload_size: 120 + i * 10,
    payload: JSON.stringify({ orderId: 20931 + i, amount: 9900 + i * 100, status: "pending" }),
    headers: { "x-trace-id": `trace-${i}` },
    redelivered: i === 0,
  }));
}

export async function createQueue(config) {
  await delay();
  return true;
}

export async function updateQueuePolicy(input) {
  await delay();
  return true;
}

export async function deleteQueue(name) {
  await delay();
  return true;
}

export async function pauseQueue(name) {
  await delay();
  return true;
}

export async function resumeQueue(name) {
  await delay();
  return true;
}

export async function publishMessage(request) {
  await delay(400);
  return { status: "confirmed" };
}

export async function listMessageFeed(filter = null) {
  await delay(300);
  if (!filter) return MESSAGE_FEED;
  return MESSAGE_FEED.filter((item) => item.status === filter.status);
}

export async function getMessageTrace(traceId) {
  await delay();
  return MESSAGE_FEED.find((item) => item.trace_id === traceId) || MESSAGE_FEED[0];
}

export async function deleteMessageTrace(traceId) {
  await delay();
  return true;
}

export async function setQueueAlertRule(rule) {
  await delay();
  return true;
}

export async function listQueueAlertRules(queueName, vhost) {
  await delay();
  return ALERT_RULES;
}

export async function deleteQueueAlertRule(queueName, vhost, metric) {
  await delay();
  return true;
}

export async function listQueueAlertRecords(queueName, vhost) {
  await delay();
  return ALERT_RECORDS;
}

export async function checkQueueAlerts() {
  await delay();
  return [];
}

export async function listQueueAuditLogs(filter = {}) {
  await delay();
  return AUDIT_LOGS;
}

export async function exportQueueAuditLogs(filter, path) {
  await delay();
  return true;
}

export async function listQueuesPaginated(filter = {}, pagination = {}) {
  await delay(300);
  let list = [...QUEUES];
  if (filter.search) {
    const kw = filter.search.toLowerCase();
    list = list.filter((q) => q.name.toLowerCase().includes(kw));
  }
  if (filter.queue_type && filter.queue_type !== "all") {
    list = list.filter((q) => q.queue_type === filter.queue_type);
  }
  if (filter.health && filter.health !== "all") {
    list = list.filter((q) => q.health === filter.health);
  }
  const page = pagination.page || 1;
  const pageSize = pagination.page_size || 50;
  const total = list.length;
  const items = list.slice((page - 1) * pageSize, page * pageSize);
  return { items, total, page, page_size: pageSize };
}

export async function listRabbitConnections(pagination = {}) {
  await delay(300);
  return { items: RABBIT_CONNECTIONS, total: RABBIT_CONNECTIONS.length, page: pagination.page || 1, page_size: pagination.page_size || 50 };
}

export async function listChannels(connectionName = null, pagination = {}) {
  await delay(300);
  const items = connectionName ? CHANNELS[connectionName] || [] : Object.values(CHANNELS).flat();
  return { items, total: items.length, page: pagination.page || 1, page_size: pagination.page_size || 50 };
}

export async function listQueueBindings(queueName) {
  await delay();
  return BINDINGS[queueName] || [];
}

export async function deleteQueueBinding(queueName, binding) {
  await delay();
  return true;
}

export async function purgeQueue(name) {
  await delay();
  const q = QUEUES.find((x) => x.name === name);
  if (q) {
    q.ready = 0;
    q.unacked = 0;
    q.total = 0;
  }
  return true;
}

export function extractErrorMessage(error) {
  if (typeof error === "string") return error;
  if (error?.message) return error.message;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

// 窗口控制在浏览器中静默失败
export async function minimizeWindow() {}
export async function toggleMaximizeWindow() {}
export async function closeWindow() {}
export async function startDragging() {}

// 事件监听在浏览器中无需处理
export async function listenQueueRefreshed(callback) {
  return () => {};
}

export async function listenManagementStale(callback) {
  return () => {};
}

export async function setRefreshEnabled(enabled) {
  await delay();
  return true;
}

export async function setRefreshInterval(ms) {
  await delay();
  return true;
}
