//! 手动消费者管理器（消费者工作室）
//!
//! 支持：
//! - 创建 / 启动 / 暂停 / 继续 / 销毁消费者
//! - 预览模式（手动 Ack，只看不消费）
//! - 真实消费模式（自动 Ack）
//! - payload / header 消息过滤
//! - 最多保留 500 条消息

use crate::error::{AppError, AppResult};
use crate::models::{ConsumerMessage, ManualConsumer, ManualConsumerConfig};
use futures_util::StreamExt;
use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions};
use lapin::types::{AMQPValue, FieldTable};
use lapin::{Channel, Connection, ConnectionProperties};
use parking_lot::RwLock;
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

const MAX_MESSAGES: usize = 500;

pub struct ConsumerManager {
    consumers: RwLock<HashMap<String, Arc<RwLock<ConsumerEntry>>>>,
}

struct ConsumerEntry {
    id: String,
    config: ManualConsumerConfig,
    status: String,
    error: Option<String>,
    consumed_count: u64,
    filtered_count: u64,
    runtime: Option<ConsumerRuntime>,
    messages: Arc<RwLock<VecDeque<ConsumerMessage>>>,
    pending_acks: Arc<RwLock<HashMap<u64, DeliveryAck>>>,
    /// message_id -> delivery_tag，用于前端逐条 Ack
    message_tags: Arc<RwLock<HashMap<String, u64>>>,
}

struct ConsumerRuntime {
    connection: Connection,
    channel: Channel,
    consumer_tag: String,
    _handle: JoinHandle<()>,
}

struct DeliveryAck {
    channel: Channel,
}

impl ConsumerManager {
    pub fn new() -> Self {
        Self {
            consumers: RwLock::new(HashMap::new()),
        }
    }

    /// 创建消费者（不启动），默认进入 pending 状态。
    pub fn create(&self, config: ManualConsumerConfig) -> AppResult<ManualConsumer> {
        validate_config(&config)?;
        let id = Uuid::new_v4().to_string();
        let entry = ConsumerEntry {
            id: id.clone(),
            config,
            status: "pending".to_string(),
            error: None,
            consumed_count: 0,
            filtered_count: 0,
            runtime: None,
            messages: Arc::new(RwLock::new(VecDeque::new())),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            message_tags: Arc::new(RwLock::new(HashMap::new())),
        };
        self.consumers
            .write()
            .insert(id.clone(), Arc::new(RwLock::new(entry)));
        Ok(self.to_manual_consumer(&id))
    }

    /// 启动或继续消费者。
    pub async fn start(&self, id: &str, amqp_url: &str) -> AppResult<ManualConsumer> {
        let entry_arc = self.get_entry(id)?;

        {
            let mut entry = entry_arc.write();
            if entry.status == "running" {
                return Ok(self.entry_to_manual_consumer(&entry));
            }
            if entry.status == "destroyed" {
                return Err(AppError::ConsumerFailed("消费者已销毁".to_string()));
            }
            entry.status = "running".to_string();
            entry.error = None;
        }

        match self.spawn_consumer(id, amqp_url, entry_arc.clone()).await {
            Ok(runtime) => {
                let mut entry = entry_arc.write();
                entry.runtime = Some(runtime);
                Ok(self.entry_to_manual_consumer(&entry))
            }
            Err(e) => {
                let mut entry = entry_arc.write();
                entry.status = "error".to_string();
                entry.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// 暂停消费者：未确认消息重新入队，关闭 channel/连接。
    pub async fn pause(&self, id: &str) -> AppResult<ManualConsumer> {
        let entry_arc = self.get_entry(id)?;
        self.nack_all_pending(&entry_arc).await;
        self.cancel_runtime(&entry_arc).await?;
        {
            let mut entry = entry_arc.write();
            if entry.status == "running" {
                entry.status = "paused".to_string();
            }
        }
        Ok(self.to_manual_consumer(id))
    }

    /// 继续消费（等价于 start）。
    pub async fn resume(&self, id: &str, amqp_url: &str) -> AppResult<ManualConsumer> {
        self.start(id, amqp_url).await
    }

    /// 销毁消费者：未确认消息重新入队，清理资源。
    pub async fn destroy(&self, id: &str) -> AppResult<()> {
        let entry_arc = self.get_entry(id)?;
        self.nack_all_pending(&entry_arc).await;
        self.cancel_runtime(&entry_arc).await?;
        {
            let mut entry = entry_arc.write();
            entry.status = "destroyed".to_string();
            entry.error = None;
            entry.runtime = None;
            entry.pending_acks.write().clear();
            entry.message_tags.write().clear();
        }
        self.consumers.write().remove(id);
        Ok(())
    }

    pub fn get(&self, id: &str) -> AppResult<ManualConsumer> {
        Ok(self.to_manual_consumer(id))
    }

    pub fn list(&self) -> Vec<ManualConsumer> {
        let guard = self.consumers.read();
        guard.keys().map(|id| self.to_manual_consumer(id)).collect()
    }

    pub fn list_messages(&self, id: &str, limit: usize) -> AppResult<Vec<ConsumerMessage>> {
        let entry_arc = self.get_entry(id)?;
        let entry = entry_arc.read();
        let messages = entry.messages.read();
        let result: Vec<ConsumerMessage> = messages.iter().take(limit).cloned().collect();
        Ok(result)
    }

    /// 前端逐条确认（仅手动 Ack / 预览模式有效）。
    pub async fn ack_message(&self, consumer_id: &str, message_id: &str) -> AppResult<()> {
        let entry_arc = self.get_entry(consumer_id)?;

        let (delivery_tag, channel) = {
            let entry = entry_arc.read();
            if entry.status != "running" {
                return Err(AppError::ConsumerFailed(
                    "消费者未在运行，无法确认消息".to_string(),
                ));
            }
            let delivery_tag = entry
                .message_tags
                .read()
                .get(message_id)
                .copied()
                .ok_or_else(|| AppError::ConsumerFailed("消息不存在或已确认".to_string()))?;
            let channel = entry
                .runtime
                .as_ref()
                .map(|r| r.channel.clone())
                .ok_or_else(|| AppError::ConsumerFailed("消费者未运行".to_string()))?;
            let exists = entry.pending_acks.read().contains_key(&delivery_tag);
            if !exists {
                return Err(AppError::ConsumerFailed("消息不存在或已确认".to_string()));
            }
            (delivery_tag, channel)
        };

        channel
            .basic_ack(delivery_tag, BasicAckOptions::default())
            .await
            .map_err(|e| AppError::ConsumerFailed(format!("确认消息失败：{e}")))?;

        let entry = entry_arc.write();
        entry.pending_acks.write().remove(&delivery_tag);
        entry.message_tags.write().remove(message_id);

        // 更新消息 acked 状态
        if let Some(msg) = entry
            .messages
            .write()
            .iter_mut()
            .find(|m| m.id == message_id)
        {
            msg.acked = true;
        }

        Ok(())
    }

    fn get_entry(&self, id: &str) -> AppResult<Arc<RwLock<ConsumerEntry>>> {
        self.consumers
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::ConsumerFailed(format!("消费者不存在：{id}")))
    }

    async fn spawn_consumer(
        &self,
        id: &str,
        amqp_url: &str,
        entry_arc: Arc<RwLock<ConsumerEntry>>,
    ) -> AppResult<ConsumerRuntime> {
        let (queue_name, auto_ack, prefetch_count, mode) = {
            let entry = entry_arc.read();
            (
                entry.config.queue_name.clone(),
                entry.config.auto_ack,
                entry.config.prefetch_count,
                entry.config.mode.clone(),
            )
        };

        let conn = Connection::connect(amqp_url, ConnectionProperties::default())
            .await
            .map_err(|e| AppError::ConsumerFailed(format!("AMQP 连接失败：{e}")))?;

        let channel = conn
            .create_channel()
            .await
            .map_err(|e| AppError::ConsumerFailed(format!("创建 channel 失败：{e}")))?;

        if mode == "async" {
            channel
                .basic_qos(
                    prefetch_count,
                    BasicQosOptions {
                        global: false,
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| AppError::ConsumerFailed(format!("设置 prefetch 失败：{e}")))?;
        }

        let consumer_tag = format!("mqdesk-{id}");
        let mut consumer = channel
            .basic_consume(
                &queue_name,
                &consumer_tag,
                BasicConsumeOptions {
                    no_ack: false,
                    ..BasicConsumeOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::ConsumerFailed(format!("启动消费失败：{e}")))?;

        let messages = entry_arc.read().messages.clone();
        let pending_acks = entry_arc.read().pending_acks.clone();
        let message_tags = entry_arc.read().message_tags.clone();
        let entry_for_task = entry_arc.clone();
        let id_for_task = id.to_string();
        let channel_for_ack = channel.clone();

        let handle = tokio::spawn(async move {
            while let Some(delivery_result) = consumer.next().await {
                match delivery_result {
                    Ok(delivery) => {
                        Self::handle_delivery(
                            &delivery,
                            &entry_for_task,
                            &id_for_task,
                            &channel_for_ack,
                            &messages,
                            &pending_acks,
                            &message_tags,
                            auto_ack,
                        )
                        .await;
                    }
                    Err(e) => {
                        let mut entry = entry_for_task.write();
                        if entry.status == "running" {
                            entry.status = "error".to_string();
                            entry.error = Some(format!("消费流错误：{e}"));
                        }
                    }
                }
            }

            let mut entry = entry_for_task.write();
            if entry.status == "running" {
                entry.status = "paused".to_string();
            }
            entry.runtime = None;
        });

        Ok(ConsumerRuntime {
            connection: conn,
            channel,
            consumer_tag,
            _handle: handle,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_delivery(
        delivery: &Delivery,
        entry_arc: &Arc<RwLock<ConsumerEntry>>,
        consumer_id: &str,
        channel: &Channel,
        messages: &Arc<RwLock<VecDeque<ConsumerMessage>>>,
        pending_acks: &Arc<RwLock<HashMap<u64, DeliveryAck>>>,
        message_tags: &Arc<RwLock<HashMap<String, u64>>>,
        auto_ack: bool,
    ) {
        let payload = String::from_utf8_lossy(&delivery.data).to_string();
        let headers = field_table_to_json(delivery.properties.headers().as_ref());

        let should_pass = {
            let entry = entry_arc.read();
            filter_matches(&entry.config.filter, &payload, &headers)
        };

        if should_pass {
            let message_id = Uuid::new_v4().to_string();
            let msg = ConsumerMessage {
                id: message_id.clone(),
                consumer_id: consumer_id.to_string(),
                timestamp_ms: now_ms(),
                exchange: delivery.exchange.to_string(),
                routing_key: delivery.routing_key.to_string(),
                payload: payload.clone(),
                payload_size: delivery.data.len() as u64,
                headers,
                redelivered: delivery.redelivered,
                acked: auto_ack,
            };

            {
                let mut msgs = messages.write();
                msgs.push_front(msg);
                if msgs.len() > MAX_MESSAGES {
                    msgs.pop_back();
                }
            }

            if auto_ack {
                let _ = channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await;
            } else {
                pending_acks.write().insert(
                    delivery.delivery_tag,
                    DeliveryAck {
                        channel: channel.clone(),
                    },
                );
                message_tags
                    .write()
                    .insert(message_id, delivery.delivery_tag);
            }

            let mut entry = entry_arc.write();
            entry.consumed_count += 1;
        } else {
            let _ = channel
                .basic_nack(
                    delivery.delivery_tag,
                    BasicNackOptions {
                        multiple: false,
                        requeue: true,
                    },
                )
                .await;
            let mut entry = entry_arc.write();
            entry.filtered_count += 1;
        }
    }

    async fn nack_all_pending(&self, entry_arc: &Arc<RwLock<ConsumerEntry>>) {
        let pending: Vec<(u64, DeliveryAck)> = {
            let entry = entry_arc.read();
            let acks = entry.pending_acks.read();
            acks.iter().map(|(k, v)| (*k, v.clone())).collect()
        };

        for (tag, ack) in pending {
            let _ = ack
                .channel
                .basic_nack(
                    tag,
                    BasicNackOptions {
                        multiple: false,
                        requeue: true,
                    },
                )
                .await;
        }

        let entry = entry_arc.read();
        entry.pending_acks.write().clear();
        entry.message_tags.write().clear();
    }

    async fn cancel_runtime(&self, entry_arc: &Arc<RwLock<ConsumerEntry>>) -> AppResult<()> {
        let runtime = {
            let mut entry = entry_arc.write();
            entry.runtime.take()
        };

        if let Some(runtime) = runtime {
            let _ = runtime
                .channel
                .basic_cancel(&runtime.consumer_tag, BasicCancelOptions::default())
                .await;
            let _ = runtime.channel.close(0, "consumer paused/destroyed").await;
            let _ = runtime.connection.close(0, "consumer paused/destroyed").await;
        }

        Ok(())
    }

    fn to_manual_consumer(&self, id: &str) -> ManualConsumer {
        let entry_arc = self.get_entry(id).expect("entry exists");
        let entry = entry_arc.read();
        self.entry_to_manual_consumer(&entry)
    }

    fn entry_to_manual_consumer(&self, entry: &ConsumerEntry) -> ManualConsumer {
        ManualConsumer {
            id: entry.id.clone(),
            name: entry.config.name.clone(),
            queue_name: entry.config.queue_name.clone(),
            mode: entry.config.mode.clone(),
            prefetch_count: entry.config.prefetch_count,
            auto_ack: entry.config.auto_ack,
            status: entry.status.clone(),
            error: entry.error.clone(),
            consumed_count: entry.consumed_count,
            filtered_count: entry.filtered_count,
        }
    }
}

fn validate_config(config: &ManualConsumerConfig) -> AppResult<()> {
    if config.name.trim().is_empty() {
        return Err(AppError::InvalidConnection("消费者名称不能为空".to_string()));
    }
    if config.queue_name.trim().is_empty() {
        return Err(AppError::InvalidConnection("队列名称不能为空".to_string()));
    }
    if config.mode != "sync" && config.mode != "async" {
        return Err(AppError::InvalidConnection(
            "处理模式必须是 sync 或 async".to_string(),
        ));
    }
    if !config.filter.payload_value.is_empty() {
        match config.filter.payload_type.as_str() {
            "contains" | "equals" => {}
            "regex" => {
                let _ = Regex::new(&config.filter.payload_value)
                    .map_err(|e| AppError::InvalidConnection(format!("正则表达式无效：{e}")))?;
            }
            _ => {
                return Err(AppError::InvalidConnection(
                    "payload 过滤类型无效".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn filter_matches(
    filter: &crate::models::ConsumerFilter,
    payload: &str,
    headers: &serde_json::Value,
) -> bool {
    if !filter.payload_value.is_empty() {
        let matched = match filter.payload_type.as_str() {
            "contains" => payload.contains(&filter.payload_value),
            "equals" => payload == filter.payload_value,
            "regex" => Regex::new(&filter.payload_value)
                .map(|re| re.is_match(payload))
                .unwrap_or(false),
            _ => true,
        };
        if !matched {
            return false;
        }
    }

    for hf in &filter.headers {
        if hf.key.is_empty() {
            continue;
        }
        let header_matched = headers
            .get(&hf.key)
            .and_then(|v| v.as_str())
            .map(|s| s == hf.value)
            .unwrap_or(false);
        if !header_matched {
            return false;
        }
    }

    true
}

fn field_table_to_json(ft: Option<&FieldTable>) -> serde_json::Value {
    let Some(ft) = ft else {
        return serde_json::Value::Object(serde_json::Map::new());
    };
    let mut map = serde_json::Map::new();
    for (k, v) in ft {
        map.insert(k.to_string(), amqp_value_to_json(v));
    }
    serde_json::Value::Object(map)
}

fn amqp_value_to_json(value: &AMQPValue) -> serde_json::Value {
    match value {
        AMQPValue::Boolean(b) => serde_json::Value::Bool(*b),
        AMQPValue::ShortShortInt(n) => serde_json::Value::Number((*n as i64).into()),
        AMQPValue::ShortShortUInt(n) => serde_json::Value::Number((*n as u64).into()),
        AMQPValue::ShortInt(n) => serde_json::Value::Number((*n as i64).into()),
        AMQPValue::ShortUInt(n) => serde_json::Value::Number((*n as u64).into()),
        AMQPValue::LongInt(n) => serde_json::Value::Number((*n).into()),
        AMQPValue::LongUInt(n) => serde_json::Value::Number((*n as u64).into()),
        AMQPValue::LongLongInt(n) => serde_json::Value::Number((*n).into()),
        AMQPValue::Float(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n as f64).unwrap_or(0.into())),
        AMQPValue::Double(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(0.into())),
        AMQPValue::DecimalValue(_) => serde_json::Value::String(format!("{:?}", value)),
        AMQPValue::ShortString(s) => serde_json::Value::String(s.to_string()),
        AMQPValue::LongString(s) => serde_json::Value::String(s.to_string()),
        AMQPValue::FieldArray(a) => {
            serde_json::Value::Array(a.as_slice().iter().map(amqp_value_to_json).collect())
        }
        AMQPValue::Timestamp(n) => serde_json::Value::Number((*n).into()),
        AMQPValue::FieldTable(t) => field_table_to_json(Some(t)),
        AMQPValue::ByteArray(b) => serde_json::Value::String(format!("{:?}", b.as_slice())),
        AMQPValue::Void => serde_json::Value::Null,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Clone for DeliveryAck {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
        }
    }
}
