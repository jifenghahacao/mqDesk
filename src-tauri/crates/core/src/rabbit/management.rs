//! RabbitMQ Management HTTP API 客户端（端口 15672）
//!
//! 覆盖：
//! - 总览（/api/overview）
//! - 队列列表/详情（/api/queues, /api/queues/{vhost}/{name}）
//! - 交换机列表（/api/exchanges）
//! - 抓取预览（POST /api/queues/{vhost}/{name}/get，requeue=true）
//! - 发布（POST /api/exchanges/{vhost}/{name}/publish）

use crate::error::{AppError, AppResult};
use crate::models::{
    BindingInfo, ChannelInfo, ConnectionInfo, ConsumerInfo, NodeInfo, Paginated, Pagination, PreviewMessage,
    QueueConnectionInfo, QueueDetail, QueueFilter, QueueMessage, QueueSummary, RateHistory,
};
use crate::rabbit::rate_limiter::ManagementRateLimiter;
use base64::Engine;
use parking_lot::RwLock;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct ManagementClient {
    client: Client,
    base_url: String,
    auth_header: String,
    vhost: String,
    rate_limiter: Arc<ManagementRateLimiter>,
    is_stale: AtomicBool,
    overview_cache: RwLock<Option<ManagementOverview>>,
    nodes_cache: RwLock<Option<Vec<ManagementNode>>>,
    consumers_cache: RwLock<Option<Vec<ConsumerInfo>>>,
    exchanges_cache: RwLock<Option<Vec<ManagementExchange>>>,
}

impl ManagementClient {
    pub fn new(
        scheme: &str,
        host: &str,
        port: u16,
        vhost: &str,
        username: &str,
        password: &str,
    ) -> Self {
        let base_url = format!("{scheme}://{host}:{port}");
        // reqwest 默认支持 Basic Auth，但为了简单可控，直接构造 header
        let credentials = format!("{username}:{password}");
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("无法构建 reqwest Client");

        Self {
            client,
            base_url,
            auth_header,
            vhost: vhost.to_string(),
            rate_limiter: Arc::new(ManagementRateLimiter::default()),
            is_stale: AtomicBool::new(false),
            overview_cache: RwLock::new(None),
            nodes_cache: RwLock::new(None),
            consumers_cache: RwLock::new(None),
            exchanges_cache: RwLock::new(None),
        }
    }

    /// 当前是否处于降级（stale）状态
    pub fn is_stale(&self) -> bool {
        self.is_stale.load(Ordering::SeqCst)
    }

    async fn _get_json_raw<T: for<'de> Deserialize<'de>>(&self, path: &str) -> AppResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::AuthFailed);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ManagementResponse {
                status: status.as_u16(),
                body,
            });
        }

        let body: T = response.json().await?;
        Ok(body)
    }

    /// 带限流的 GET 请求
    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> AppResult<T> {
        self.rate_limiter.acquire().await;
        self._get_json_raw(path).await
    }

    /// 带限流与降级缓存的 GET 请求
    async fn get_json_with_fallback<T: for<'de> Deserialize<'de> + Clone>(
        &self,
        path: &str,
        cache: &RwLock<Option<T>>,
    ) -> AppResult<T> {
        self.rate_limiter.acquire().await;
        match self._get_json_raw::<T>(path).await {
            Ok(data) => {
                *cache.write() = Some(data.clone());
                self.is_stale.store(false, Ordering::SeqCst);
                Ok(data)
            }
            Err(e) => {
                if let Some(cached) = cache.read().clone() {
                    log::warn!("Management API 失败，返回缓存数据: {}", e);
                    self.is_stale.store(true, Ordering::SeqCst);
                    Ok(cached)
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn post_json<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &B,
    ) -> AppResult<T> {
        self.rate_limiter.acquire().await;
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::AuthFailed);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ManagementResponse {
                status: status.as_u16(),
                body,
            });
        }

        let body: T = response.json().await?;
        Ok(body)
    }

    async fn put_json<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> AppResult<()> {
        self.rate_limiter.acquire().await;
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::AuthFailed);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ManagementResponse {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    async fn delete_no_body(&self, path: &str) -> AppResult<()> {
        self.rate_limiter.acquire().await;
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::AuthFailed);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ManagementResponse {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    /// 测试连接：GET /api/whoami
    pub async fn test_connection(&self) -> AppResult<String> {
        let resp: WhoAmI = self.get_json("/api/whoami").await?;
        Ok(resp.name)
    }

    /// 拉取总览数据（带降级缓存）
    pub async fn get_overview_stats(&self) -> AppResult<ManagementOverview> {
        self.get_json_with_fallback("/api/overview", &self.overview_cache)
            .await
    }

    /// 列出当前 vhost 的队列，支持分页与客户端过滤（服务端只支持 name 过滤）
    pub async fn list_queues(
        &self,
        filter: &QueueFilter,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ManagementQueue>> {
        let search = filter.search.trim().to_lowercase();
        let path = format!(
            "/api/queues/{}?page={}&page_size={}",
            urlencoding(&self.vhost),
            pagination.page,
            pagination.page_size
        );
        // 分页结果不适合写入完整缓存，避免污染 queues_cache
        let response: QueuesResponse = self.get_json(&path).await?;
        // 列表拉取成功说明 Management API 已恢复，清除 stale 标记
        self.is_stale.store(false, Ordering::SeqCst);
        let (mut queues, mut total) = match response {
            QueuesResponse::Array(qs) => {
                let total = qs.len() as u64;
                (qs, total)
            }
            QueuesResponse::Paginated(p) => (p.items, p.filtered_count.max(p.total_count)),
        };

        if !search.is_empty() {
            queues.retain(|q| q.name.to_lowercase().contains(&search));
        }
        if !filter.queue_type.is_empty() && filter.queue_type != "all" {
            queues.retain(|q| q.queue_type == filter.queue_type);
        }

        // 如果服务端未返回有效总数（纯数组或旧版响应），用过滤后的当前页数量兜底。
        if total == 0 {
            total = queues.len() as u64;
        }

        Ok(Paginated {
            items: queues,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
        })
    }

    /// 取队列详情
    pub async fn get_queue(&self, name: &str) -> AppResult<ManagementQueue> {
        let path = format!(
            "/api/queues/{}/{}",
            urlencoding(&self.vhost),
            urlencoding(name)
        );
        self.get_json(&path).await
    }

    /// 声明（创建）队列
    pub async fn create_queue(&self, input: &crate::models::CreateQueueInput) -> AppResult<()> {
        let path = format!(
            "/api/queues/{}/{}",
            urlencoding(&input.vhost),
            urlencoding(&input.name)
        );
        let mut body = serde_json::Map::new();
        body.insert("durable".to_string(), input.durable.into());
        body.insert("auto_delete".to_string(), input.auto_delete.into());
        body.insert("arguments".to_string(), input.arguments.clone());
        body.insert("vhost".to_string(), input.vhost.clone().into());
        body.insert("name".to_string(), input.name.clone().into());
        if input.queue_type == "quorum" {
            let mut args = match body.get("arguments") {
                Some(serde_json::Value::Object(m)) => m.clone(),
                _ => serde_json::Map::new(),
            };
            args.insert("x-queue-type".to_string(), "quorum".into());
            body.insert("arguments".to_string(), args.into());
        }
        self.put_json(&path, &body).await
    }

    /// 删除队列
    pub async fn delete_queue(&self, name: &str) -> AppResult<()> {
        let path = format!(
            "/api/queues/{}/{}",
            urlencoding(&self.vhost),
            urlencoding(name)
        );
        self.delete_no_body(&path).await
    }

    /// 暂停队列（仅 classic 队列支持）
    pub async fn pause_queue(&self, name: &str) -> AppResult<()> {
        let path = format!(
            "/api/queues/{}/{}",
            urlencoding(&self.vhost),
            urlencoding(name)
        );
        let body = serde_json::json!({
            "paused": true,
            "durable": true,
            "auto_delete": false,
            "arguments": {},
            "vhost": self.vhost,
            "name": name,
        });
        self.put_json(&path, &body).await
    }

    /// 恢复队列
    pub async fn resume_queue(&self, name: &str) -> AppResult<()> {
        let path = format!(
            "/api/queues/{}/{}",
            urlencoding(&self.vhost),
            urlencoding(name)
        );
        let body = serde_json::json!({
            "paused": false,
            "durable": true,
            "auto_delete": false,
            "arguments": {},
            "vhost": self.vhost,
            "name": name,
        });
        self.put_json(&path, &body).await
    }

    /// 清空队列消息（保留队列元数据）
    pub async fn purge_queue(&self, name: &str) -> AppResult<()> {
        let path = format!(
            "/api/queues/{}/{}/contents",
            urlencoding(&self.vhost),
            urlencoding(name)
        );
        self.delete_no_body(&path).await
    }

    /// 列出指定队列的绑定
    pub async fn list_queue_bindings(&self, queue_name: &str) -> AppResult<Vec<BindingInfo>> {
        let path = format!(
            "/api/queues/{}/{}/bindings",
            urlencoding(&self.vhost),
            urlencoding(queue_name)
        );
        let raw: Vec<ManagementBinding> = self.get_json(&path).await?;
        Ok(raw
            .into_iter()
            .map(|b| BindingInfo {
                source: b.source,
                vhost: b.vhost,
                destination: b.destination,
                destination_type: b.destination_type,
                routing_key: b.routing_key,
                arguments: b.arguments,
                properties_key: b.properties_key,
            })
            .collect())
    }

    /// 删除绑定
    ///
    /// RabbitMQ API: DELETE /api/bindings/{vhost}/e/{source}/{destination_type}/{destination}/{properties_key}
    pub async fn delete_binding(&self, binding: &BindingInfo) -> AppResult<()> {
        // Management API 删除路径中 destination_type 需用缩写：queue -> q，exchange -> e
        let dest_type = match binding.destination_type.as_str() {
            "queue" => "q",
            "exchange" => "e",
            other => other,
        };
        let path = format!(
            "/api/bindings/{}/e/{}/{}/{}/{}",
            urlencoding(&binding.vhost),
            urlencoding(&binding.source),
            dest_type,
            urlencoding(&binding.destination),
            urlencoding(&binding.properties_key)
        );
        self.delete_no_body(&path).await
    }

    /// 通过 Policy 修改队列可改参数
    pub async fn update_queue_policy(
        &self,
        input: &crate::models::QueuePolicyInput,
    ) -> AppResult<()> {
        let path = format!("/api/policies/{}/{}", urlencoding(&input.vhost), urlencoding(&input.name));
        let mut definition = serde_json::Map::new();
        if let Some(n) = input.max_length {
            definition.insert("max-length".to_string(), n.into());
        }
        if let Some(n) = input.message_ttl {
            definition.insert("message-ttl".to_string(), n.into());
        }
        if let Some(ref dlx) = input.dead_letter_exchange {
            definition.insert("dead-letter-exchange".to_string(), dlx.clone().into());
        }
        if let Some(ref dlkey) = input.dead_letter_routing_key {
            definition.insert("dead-letter-routing-key".to_string(), dlkey.clone().into());
        }
        if definition.is_empty() {
            return Ok(());
        }
        let body = serde_json::json!({
            "pattern": format!("^{}$", regex::escape(&input.name)),
            "apply-to": "queues",
            "definition": definition,
            "priority": 1,
            "vhost": input.vhost,
            "name": input.name,
        });
        self.put_json(&path, &body).await
    }

    /// 抓取最新 N 条消息预览（requeue=true，不真正消费）
    pub async fn get_messages_preview(
        &self,
        queue_name: &str,
        count: u32,
    ) -> AppResult<Vec<PreviewMessage>> {
        let raw = self.get_messages_raw(queue_name, count).await?;
        Ok(raw
            .into_iter()
            .map(|m| {
                // RabbitMQ 4.x 把 timestamp/headers 放到 properties 里
                let (timestamp, headers) = extract_from_properties(&m.properties, m.timestamp, m.headers);
                PreviewMessage {
                    routing_key: m.routing_key,
                    timestamp: format_timestamp(timestamp),
                    size_bytes: m.payload_bytes as u64,
                    payload_preview: decode_payload(&m.payload, &m.payload_encoding),
                    headers,
                }
            })
            .collect())
    }

    /// 抓取最新 N 条消息（完整信息，用于队列消息管理页）
    pub async fn peek_queue_messages(
        &self,
        queue_name: &str,
        count: u32,
    ) -> AppResult<Vec<QueueMessage>> {
        let raw = self.get_messages_raw(queue_name, count).await?;
        Ok(raw
            .into_iter()
            .map(|m| QueueMessage {
                delivery_tag: m.delivery_tag as u64,
                exchange: m.exchange.unwrap_or_default(),
                routing_key: m.routing_key,
                payload: decode_payload(&m.payload, &m.payload_encoding),
                payload_size: m.payload_bytes as u64,
                headers: m
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("headers"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
                properties: m.properties.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
                redelivered: m.redelivered.unwrap_or(false),
            })
            .collect())
    }

    async fn get_messages_raw(
        &self,
        queue_name: &str,
        count: u32,
    ) -> AppResult<Vec<GetMessageResponse>> {
        let path = format!(
            "/api/queues/{}/{}/get",
            urlencoding(&self.vhost),
            urlencoding(queue_name)
        );
        let body = GetMessageRequest {
            count,
            ackmode: "ack_requeue_true".to_string(),
            encoding: "auto".to_string(),
            truncate: 50000,
        };
        self.post_json(&path, &body).await
    }

    /// 列出交换机（带降级缓存）
    pub async fn list_exchanges(&self) -> AppResult<Vec<ManagementExchange>> {
        let path = format!("/api/exchanges/{}", urlencoding(&self.vhost));
        self.get_json_with_fallback(&path, &self.exchanges_cache).await
    }

    /// 列出集群节点（带降级缓存）
    pub async fn list_nodes(&self) -> AppResult<Vec<NodeInfo>> {
        let raw: Vec<ManagementNode> =
            self.get_json_with_fallback("/api/nodes", &self.nodes_cache).await?;
        Ok(raw.into_iter().map(into_node_info).collect())
    }

    /// 列出当前 vhost 的活跃连接（支持分页）
    pub async fn list_connections(
        &self,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ConnectionInfo>> {
        // RabbitMQ /api/connections 返回所有 vhost 的连接；按 vhost 在客户端过滤
        let path = format!(
            "/api/connections?page={}&page_size={}",
            pagination.page, pagination.page_size
        );
        let response: ConnectionsResponse = self.get_json(&path).await?;
        let (raw, total) = match response {
            ConnectionsResponse::Array(arr) => {
                let total = arr.len() as u64;
                (arr, total)
            }
            ConnectionsResponse::Paginated(p) => (
                p.items,
                p.filtered_count.max(p.total_count),
            ),
        };
        let raw: Vec<ManagementConnection> = raw
            .into_iter()
            .filter(|c| c.vhost == self.vhost)
            .collect();
        let total = total.max(raw.len() as u64);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let items: Vec<ConnectionInfo> = raw
            .into_iter()
            .map(|c| {
                let connected_seconds = if c.connected_at > 0 {
                    now_ms.saturating_sub(c.connected_at) / 1000
                } else {
                    0
                };
                ConnectionInfo {
                    name: c.name.clone(),
                    peer_host: c.peer_host.clone(),
                    peer_port: c.peer_port,
                    peer_address: format!("{}:{}", c.peer_host, c.peer_port),
                    protocol: c.protocol.clone(),
                    connected_at: c.connected_at,
                    connected_seconds,
                    channel_count: c.channels,
                    state: c.state,
                }
            })
            .collect();

        // total 已在上方从分页响应中解析；纯数组响应用当前页条数占位。
        Ok(Paginated {
            items,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
        })
    }

    /// 列出当前 vhost 的信道（支持分页 + 按连接名过滤）
    pub async fn list_channels(
        &self,
        connection_name: Option<&str>,
        pagination: &Pagination,
    ) -> AppResult<Paginated<ChannelInfo>> {
        // RabbitMQ /api/channels 返回所有 vhost 的信道；按 vhost 在客户端过滤
        let path = format!(
            "/api/channels?page={}&page_size={}",
            pagination.page, pagination.page_size
        );
        let response: ChannelsResponse = self.get_json(&path).await?;
        let (raw, total) = match response {
            ChannelsResponse::Array(arr) => {
                let total = arr.len() as u64;
                (arr, total)
            }
            ChannelsResponse::Paginated(p) => (
                p.items,
                p.filtered_count.max(p.total_count),
            ),
        };
        let raw: Vec<ManagementChannel> = raw
            .into_iter()
            .filter(|c| c.vhost == self.vhost)
            .collect();
        let total = total.max(raw.len() as u64);

        let mut items: Vec<ChannelInfo> = raw
            .into_iter()
            .map(|c| {
                let stats = c.message_stats.as_ref();
                ChannelInfo {
                    name: c.name.clone(),
                    connection_name: c.connection_details.name.clone(),
                    number: c.number,
                    consumer_count: c.consumer_count,
                    prefetch_count: c.prefetch_count,
                    unacked: c.messages_unacknowledged,
                    publish_rate: stats
                        .and_then(|s| s.publish_details.as_ref())
                        .map(|d| d.rate)
                        .unwrap_or(0.0),
                    deliver_rate: stats
                        .and_then(|s| s.deliver_get_details.as_ref())
                        .map(|d| d.rate)
                        .unwrap_or(0.0),
                    ack_rate: stats
                        .and_then(|s| s.ack_details.as_ref())
                        .map(|d| d.rate)
                        .unwrap_or(0.0),
                }
            })
            .collect();

        if let Some(conn_name) = connection_name {
            items.retain(|c| c.connection_name == conn_name);
        }

        Ok(Paginated {
            items,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
        })
    }

    /// 列出当前 vhost 的消费者，并补充连接时长与队列消费速率（带降级缓存）
    pub async fn list_consumers(&self) -> AppResult<Vec<ConsumerInfo>> {
        match self._list_consumers_inner().await {
            Ok(consumers) => {
                *self.consumers_cache.write() = Some(consumers.clone());
                self.is_stale.store(false, Ordering::SeqCst);
                Ok(consumers)
            }
            Err(e) => {
                if let Some(cached) = self.consumers_cache.read().clone() {
                    log::warn!("Management API 失败，返回缓存的消费者数据: {}", e);
                    self.is_stale.store(true, Ordering::SeqCst);
                    Ok(cached)
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn _list_consumers_inner(&self) -> AppResult<Vec<ConsumerInfo>> {
        let path = format!("/api/consumers/{}", urlencoding(&self.vhost));
        let raw: Vec<ManagementConsumer> = self.get_json(&path).await?;
        let connections: Vec<ManagementConnection> = self.get_json("/api/connections").await?;
        let conn_map: HashMap<String, u64> = connections
            .into_iter()
            .map(|c| (c.name, c.connected_at))
            .collect();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 批量拉取队列速率，避免每个消费者触发一次 get_queue 的 N+1 查询
        let queues_path = format!("/api/queues/{}", urlencoding(&self.vhost));
        let queues: Vec<ManagementQueue> = self.get_json(&queues_path).await?;
        let queue_rates: HashMap<String, f64> = queues
            .into_iter()
            .map(|q| {
                let rate = q
                    .message_stats
                    .as_ref()
                    .and_then(|s| s.deliver_get_details.as_ref())
                    .map(|d| d.rate)
                    .unwrap_or(0.0);
                (q.name, rate)
            })
            .collect();

        Ok(raw
            .into_iter()
            .map(|c| {
                let connected_at = c
                    .channel_details
                    .connection_name
                    .as_ref()
                    .and_then(|name| conn_map.get(name).copied())
                    .unwrap_or(0);
                let connected_seconds = if connected_at > 0 {
                    now_ms.saturating_sub(connected_at) / 1000
                } else {
                    0
                };
                let client_address = format!(
                    "{}:{}",
                    c.channel_details.peer_host.as_deref().unwrap_or("-"),
                    c.channel_details.peer_port
                );
                let message_rate = *queue_rates.get(&c.queue.name).unwrap_or(&0.0);
                ConsumerInfo {
                    consumer_tag: c.consumer_tag,
                    queue_name: c.queue.name,
                    client_address,
                    connection_name: c.channel_details.connection_name.unwrap_or_default(),
                    ack_required: c.ack_required,
                    prefetch_count: c.prefetch_count,
                    connected_seconds,
                    message_rate,
                }
            })
            .collect())
    }
}

// === 辅助 ===

fn urlencoding(s: &str) -> String {
    // RabbitMQ vhost "/" 需编码为 %2F，其他特殊字符也需编码
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    out
}

fn format_timestamp(ts: Option<i64>) -> String {
    match ts {
        Some(t) => chrono::DateTime::from_timestamp(t, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| t.to_string()),
        None => String::new(),
    }
}

/// RabbitMQ 4.x 把 timestamp/headers 移到了 properties 对象里，优先从 properties 取
fn extract_from_properties(
    properties: &Option<serde_json::Value>,
    top_timestamp: Option<i64>,
    top_headers: Option<serde_json::Value>,
) -> (Option<i64>, serde_json::Value) {
    let timestamp = properties
        .as_ref()
        .and_then(|p| p.get("timestamp"))
        .and_then(|t| t.as_i64())
        .or(top_timestamp);

    let headers = properties
        .as_ref()
        .and_then(|p| p.get("headers"))
        .cloned()
        .or(top_headers)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    (timestamp, headers)
}

fn decode_payload(payload: &str, encoding: &str) -> String {
    if encoding == "base64" {
        use base64::Engine;
        match base64::engine::general_purpose::STANDARD.decode(payload) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => payload.to_string(),
        }
    } else {
        payload.to_string()
    }
}

// === Management API 响应类型 ===

#[derive(Debug, Clone, Deserialize)]
pub struct WhoAmI {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementOverview {
    pub object_totals: ObjectTotals,
    pub queue_totals: Option<QueueTotals>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectTotals {
    pub queues: u64,
    pub exchanges: u64,
    pub connections: u64,
    pub consumers: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueTotals {
    pub messages: u64,
    pub messages_ready: u64,
    pub messages_unacknowledged: u64,
}

/// RabbitMQ Management API 带分页参数时的队列列表响应。
#[derive(Debug, Clone, Deserialize)]
struct PaginatedQueuesResponse {
    pub items: Vec<ManagementQueue>,
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub filtered_count: u64,
}

/// 兼容旧版/不带分页参数时返回的纯数组与新版分页对象两种格式。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum QueuesResponse {
    Array(Vec<ManagementQueue>),
    Paginated(PaginatedQueuesResponse),
}

#[derive(Debug, Clone, Deserialize)]
struct PaginatedConnectionsResponse {
    pub items: Vec<ManagementConnection>,
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub filtered_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ConnectionsResponse {
    Array(Vec<ManagementConnection>),
    Paginated(PaginatedConnectionsResponse),
}

#[derive(Debug, Clone, Deserialize)]
struct PaginatedChannelsResponse {
    pub items: Vec<ManagementChannel>,
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub filtered_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ChannelsResponse {
    Array(Vec<ManagementChannel>),
    Paginated(PaginatedChannelsResponse),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementQueue {
    pub name: String,
    pub vhost: String,
    #[serde(rename = "type", default)]
    pub queue_type: String,
    #[serde(default)]
    pub durable: bool,
    #[serde(rename = "auto_delete", default)]
    pub auto_delete: bool,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub policy: Option<String>,
    /// RabbitMQ 4.x 空队列响应可能省略这些字段，必须 #[serde(default)]
    #[serde(default)]
    pub messages: u64,
    #[serde(default)]
    pub messages_ready: u64,
    #[serde(default)]
    pub messages_unacknowledged: u64,
    #[serde(default)]
    pub consumers: u64,
    #[serde(default)]
    pub message_stats: Option<MessageStats>,
    #[serde(default)]
    pub consumer_details: Vec<ManagementConsumer>,
    #[serde(default)]
    pub incoming: Vec<ManagementPublisher>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementPublisher {
    pub stats: Option<MessageStats>,
    pub channel_details: ChannelDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageStats {
    pub publish: Option<u64>,
    pub deliver_get: Option<u64>,
    pub ack: Option<u64>,
    pub publish_details: Option<RateDetails>,
    pub deliver_get_details: Option<RateDetails>,
    #[serde(default)]
    pub ack_details: Option<RateDetails>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateDetails {
    pub rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementExchange {
    pub name: String,
    pub kind: String,
    pub durable: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ManagementBinding {
    pub source: String,
    pub vhost: String,
    pub destination: String,
    #[serde(rename = "destination_type")]
    pub destination_type: String,
    pub routing_key: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(rename = "properties_key")]
    pub properties_key: String,
}

#[derive(Debug, Serialize)]
struct GetMessageRequest {
    count: u32,
    ackmode: String,
    encoding: String,
    truncate: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct GetMessageResponse {
    #[serde(default)]
    pub delivery_tag: i64,
    pub payload: String,
    pub payload_bytes: i64,
    pub payload_encoding: String,
    pub routing_key: String,
    #[serde(default)]
    pub exchange: Option<String>,
    /// RabbitMQ 4.x 把 timestamp 放到 properties 里，顶层不返回
    #[serde(default)]
    pub timestamp: Option<i64>,
    /// RabbitMQ 4.x 把 headers 放到 properties 里，顶层不返回
    #[serde(default)]
    pub headers: Option<serde_json::Value>,
    /// RabbitMQ 4.x 返回 properties 对象（含 headers/timestamp/content_type 等）
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub redelivered: Option<bool>,
}

// === 集群节点 ===

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementNode {
    pub name: String,
    #[serde(default)]
    pub running: bool,
    #[serde(rename = "type", default)]
    pub node_type: String,
    #[serde(default)]
    pub uptime: u64,
    #[serde(default)]
    pub mem_used: u64,
    #[serde(default)]
    pub mem_limit: u64,
    #[serde(default)]
    pub disk_free: u64,
    #[serde(default)]
    pub disk_free_limit: u64,
    #[serde(default)]
    pub fd_used: u64,
    #[serde(default)]
    pub fd_total: u64,
    #[serde(default)]
    pub proc_used: u64,
    #[serde(default)]
    pub proc_total: u64,
    #[serde(default)]
    pub sockets_used: u64,
    #[serde(default)]
    pub sockets_total: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementConsumer {
    pub consumer_tag: String,
    pub channel_details: ChannelDetails,
    pub queue: QueueRef,
    #[serde(default)]
    pub ack_required: bool,
    #[serde(default)]
    pub prefetch_count: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChannelDetails {
    pub name: String,
    #[serde(default)]
    pub peer_host: Option<String>,
    #[serde(default)]
    pub peer_port: u16,
    #[serde(default)]
    pub connection_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueRef {
    pub name: String,
    pub vhost: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementConnection {
    pub name: String,
    #[serde(default)]
    pub vhost: String,
    #[serde(default)]
    pub peer_host: String,
    #[serde(default)]
    pub peer_port: u16,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub connected_at: u64,
    #[serde(default)]
    pub channels: u32,
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ManagementChannel {
    pub name: String,
    #[serde(default)]
    pub vhost: String,
    /// RabbitMQ 3.13+ 在 channel 对象里使用 `connection_details`，而不是顶层 `connection_name`
    #[serde(default)]
    pub connection_details: ChannelDetails,
    #[serde(default)]
    pub number: u16,
    #[serde(default)]
    pub consumer_count: u32,
    #[serde(default)]
    pub prefetch_count: u16,
    #[serde(default, rename = "messages_unacknowledged")]
    pub messages_unacknowledged: u64,
    #[serde(default)]
    pub message_stats: Option<MessageStats>,
}

fn into_node_info(n: ManagementNode) -> NodeInfo {
    let mem_usage_percent = if n.mem_limit > 0 {
        (n.mem_used as f64 / n.mem_limit as f64) * 100.0
    } else {
        0.0
    };
    // disk_free_limit 是 RabbitMQ 的磁盘告警阈值，不是总磁盘大小。
    // 这里用“剩余空间 / 阈值”判断健康度，而不是百分比（API 没有总磁盘）。
    let disk_free_status = if n.disk_free_limit == 0 {
        "ok".to_string()
    } else {
        let ratio = n.disk_free as f64 / n.disk_free_limit as f64;
        if ratio <= 1.0 {
            "danger".to_string()
        } else if ratio <= 2.0 {
            "warn".to_string()
        } else {
            "ok".to_string()
        }
    };
    NodeInfo {
        name: n.name,
        is_running: n.running,
        node_type: n.node_type,
        uptime_seconds: n.uptime / 1000,
        mem_used_bytes: n.mem_used,
        mem_limit_bytes: n.mem_limit,
        mem_usage_percent,
        disk_free_bytes: n.disk_free,
        disk_free_limit_bytes: n.disk_free_limit,
        disk_free_status,
        fd_used: n.fd_used,
        fd_total: n.fd_total,
        proc_used: n.proc_used,
        proc_total: n.proc_total,
        sockets_used: n.sockets_used,
        sockets_total: n.sockets_total,
    }
}

// === 从 ManagementQueue 到内部模型 ===

impl ManagementQueue {
    pub fn to_summary(&self) -> QueueSummary {
        let (incoming_rate, outgoing_rate) = self.rates();
        let health = crate::health::judge_health(
            self.messages_ready,
            self.consumers,
            incoming_rate,
            outgoing_rate,
        );
        let health_summary = crate::health::health_summary(&health);

        QueueSummary {
            name: self.name.clone(),
            vhost: self.vhost.clone(),
            queue_type: self.queue_type.clone(),
            durable: self.durable,
            auto_delete: self.auto_delete,
            ready: self.messages_ready,
            unacked: self.messages_unacknowledged,
            total: self.messages,
            consumers: self.consumers,
            health,
            health_summary,
            incoming_rate,
            outgoing_rate,
        }
    }

    pub fn to_detail(&self, history: RateHistory) -> QueueDetail {
        let producers = self
            .incoming
            .iter()
            .map(|p| QueueConnectionInfo {
                name: p.channel_details.name.clone(),
                peer_address: format_address(
                    p.channel_details.peer_host.as_deref(),
                    p.channel_details.peer_port,
                ),
                connection_name: p.channel_details.connection_name.clone().unwrap_or_default(),
                ack_required: None,
                prefetch_count: None,
                connected_at: None,
            })
            .collect();

        let consumers = self
            .consumer_details
            .iter()
            .map(|c| QueueConnectionInfo {
                name: c.consumer_tag.clone(),
                peer_address: format_address(
                    c.channel_details.peer_host.as_deref(),
                    c.channel_details.peer_port,
                ),
                connection_name: c.channel_details.connection_name.clone().unwrap_or_default(),
                ack_required: Some(c.ack_required),
                prefetch_count: Some(c.prefetch_count),
                connected_at: None,
            })
            .collect();

        QueueDetail {
            summary: self.to_summary(),
            arguments: self.arguments.clone(),
            policy: self.policy.clone(),
            rate_history: history,
            producers,
            consumers,
        }
    }

    fn rates(&self) -> (f64, f64) {
        let incoming = self
            .message_stats
            .as_ref()
            .and_then(|s| s.publish_details.as_ref())
            .map(|d| d.rate)
            .unwrap_or(0.0);
        let outgoing = self
            .message_stats
            .as_ref()
            .and_then(|s| s.deliver_get_details.as_ref())
            .map(|d| d.rate)
            .unwrap_or(0.0);
        (incoming, outgoing)
    }
}

fn format_address(host: Option<&str>, port: u16) -> String {
    match host {
        Some(h) if port > 0 => format!("{}:{}", h, port),
        Some(h) => h.to_string(),
        None => "-".to_string(),
    }
}
