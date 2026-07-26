//! 统一错误类型，序列化为前端可读的 `AppError`

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("连接不存在：{0}")]
    ConnectionNotFound(String),

    #[error("连接配置无效：{0}")]
    InvalidConnection(String),

    #[error("RabbitMQ 管理接口请求失败：{0}")]
    ManagementRequest(String),

    #[error("RabbitMQ 管理接口返回错误：{status} {body}")]
    ManagementResponse { status: u16, body: String },

    #[error("RabbitMQ 认证失败，请检查用户名/密码")]
    AuthFailed,

    #[error("AMQP 发布失败：{0}")]
    PublishFailed(String),

    #[error("消费者操作失败：{0}")]
    ConsumerFailed(String),

    #[error("消息不是合法 JSON")]
    InvalidJson,

    #[error("本地存储错误：{0}")]
    Storage(String),

    #[error("密码加密存储失败：{0}")]
    Crypto(String),

    #[error("未连接到 RabbitMQ")]
    NotConnected,

    #[error("操作超时")]
    Timeout,

    #[error("内部错误：{0}")]
    Internal(String),
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() || err.is_timeout() {
            AppError::ManagementRequest(format!("无法连接到管理接口：{err}"))
        } else if err.is_decode() {
            AppError::ManagementRequest(format!("解析响应失败：{err}"))
        } else {
            AppError::ManagementRequest(err.to_string())
        }
    }
}

impl From<sled::Error> for AppError {
    fn from(err: sled::Error) -> Self {
        AppError::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        if err.classify() == serde_json::error::Category::Syntax {
            AppError::InvalidJson
        } else {
            AppError::Internal(format!("序列化失败：{err}"))
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 3)?;
        let code = match self {
            AppError::ConnectionNotFound(_) => "connection_not_found",
            AppError::InvalidConnection(_) => "invalid_connection",
            AppError::ManagementRequest(_) => "management_request_failed",
            AppError::ManagementResponse { .. } => "management_response_error",
            AppError::AuthFailed => "auth_failed",
            AppError::PublishFailed(_) => "publish_failed",
            AppError::ConsumerFailed(_) => "consumer_failed",
            AppError::InvalidJson => "invalid_json",
            AppError::Storage(_) => "storage_error",
            AppError::Crypto(_) => "crypto_error",
            AppError::NotConnected => "not_connected",
            AppError::Timeout => "timeout",
            AppError::Internal(_) => "internal_error",
        };
        state.serialize_field("code", code)?;
        state.serialize_field("message", &self.to_string())?;
        if let AppError::ManagementResponse { status, body } = self {
            state.serialize_field("status", status)?;
            state.serialize_field("body", body)?;
        } else {
            state.serialize_field("status", &0)?;
            state.serialize_field("body", &"")?;
        }
        state.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
