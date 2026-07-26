//! AMQP Publisher：用 lapin 实现 publisher confirms + mandatory 路由
//!
//! 行为：
//! - 启用 publisher confirms（channel.confirm_select()）。
//! - mandatory=true 时，broker 找不到队列会触发 basic.return。
//! - 等待 confirm：ACK=confirmed；NACK=failed；basic.return=returned。
//! - 5 秒超时 → failed("confirm timeout")。
//!
//! 选用 AMQP 而非 Management API 的 publish 接口，原因：
//! - publish 接口无法拿到 mandatory 的 basic.return。
//! - 真实发布能拿到 confirm，状态追踪更准确。

use crate::error::{AppError, AppResult};
use crate::models::{PublishRequest, PublishResult, PublishStatus};
use lapin::{
    options::{BasicPublishOptions, ConfirmSelectOptions},
    types::{AMQPValue, FieldTable},
    BasicProperties, Connection, ConnectionProperties, publisher_confirm::Confirmation,
};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

pub struct AmqpPublisher;

impl AmqpPublisher {
    pub async fn publish(
        amqp_url: &str,
        _vhost: &str,
        exchange: Option<&str>,
        request: PublishRequest,
    ) -> AppResult<PublishResult> {
        let trace_id = Uuid::new_v4().to_string();

        let conn = Connection::connect(amqp_url, ConnectionProperties::default())
            .await
            .map_err(|e| AppError::PublishFailed(format!("连接 AMQP 失败：{e}")))?;

        let channel = conn.create_channel().await.map_err(|e| {
            AppError::PublishFailed(format!("创建 channel 失败：{e}"))
        })?;

        // 启用 publisher confirms
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .map_err(|e| AppError::PublishFailed(format!("启用 confirms 失败：{e}")))?;

        // 构造消息属性
        let mut headers = FieldTable::default();
        if let serde_json::Value::Object(map) = &request.headers {
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    headers.insert(k.clone().into(), AMQPValue::LongString(s.into()));
                } else if let Some(n) = v.as_i64() {
                    headers.insert(k.clone().into(), AMQPValue::LongLongInt(n));
                } else {
                    headers.insert(
                        k.clone().into(),
                        AMQPValue::LongString(v.to_string().into()),
                    );
                }
            }
        }

        // 注入 trace_id 到 headers 便于追踪
        headers.insert(
            "x-mqdesk-trace-id".into(),
            AMQPValue::LongString(trace_id.clone().into()),
        );

        let properties = BasicProperties::default()
            .with_content_type(request.content_type.as_str().into())
            .with_delivery_mode(request.delivery_mode)
            .with_headers(headers)
            .with_message_id(trace_id.clone().into());

        let exchange_name = exchange.unwrap_or("").to_string();
        let routing_key = request.routing_key.clone();
        let payload = request.payload.clone().into_bytes();

        let mut publish_options = BasicPublishOptions::default();
        if request.mandatory {
            publish_options.mandatory = true;
        }

        // 发布并拿到 PublisherConfirm（Future）
        let publisher_confirm = channel
            .basic_publish(
                &exchange_name,
                &routing_key,
                publish_options,
                &payload,
                properties,
            )
            .await
            .map_err(|e| AppError::PublishFailed(format!("basic_publish 失败：{e}")))?;

        // 等待 Confirmation，5 秒超时
        let result = match timeout(Duration::from_secs(5), publisher_confirm).await {
            Ok(Ok(confirmation)) => match confirmation {
                Confirmation::Ack(return_msg) => {
                    if let Some(ret) = return_msg {
                        // mandatory 触发 basic.return
                        PublishResult {
                            trace_id,
                            status: PublishStatus::Returned,
                            reply_code: ret.reply_code as i32,
                            reply_text: ret.reply_text.to_string(),
                            error: String::new(),
                        }
                    } else {
                        PublishResult {
                            trace_id,
                            status: PublishStatus::Confirmed,
                            reply_code: 0,
                            reply_text: String::new(),
                            error: String::new(),
                        }
                    }
                }
                Confirmation::Nack(_) => PublishResult {
                    trace_id,
                    status: PublishStatus::Failed,
                    reply_code: 0,
                    reply_text: String::new(),
                    error: "broker NACK".to_string(),
                },
                Confirmation::NotRequested => PublishResult {
                    trace_id,
                    status: PublishStatus::Confirmed,
                    reply_code: 0,
                    reply_text: String::new(),
                    error: String::new(),
                },
            },
            Ok(Err(e)) => PublishResult {
                trace_id,
                status: PublishStatus::Failed,
                reply_code: 0,
                reply_text: String::new(),
                error: format!("confirm 错误：{e}"),
            },
            Err(_) => PublishResult {
                trace_id,
                status: PublishStatus::Failed,
                reply_code: 0,
                reply_text: String::new(),
                error: "confirm timeout".to_string(),
            },
        };

        // 关闭连接
        let _ = conn.close(0, "publish done").await;

        Ok(result)
    }
}
