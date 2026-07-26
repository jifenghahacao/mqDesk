//! MQDesk 核心逻辑（不依赖 Tauri，可独立测试）

pub mod crypto;
pub mod error;
pub mod health;
pub mod models;
pub mod rabbit;
pub mod state;
pub mod storage;
pub mod trace;

pub use error::{AppError, AppResult};
pub use models::*;
pub use rabbit::{AmqpPublisher, ConsumerManager, ManagementClient};
pub use state::AppState;
pub use storage::Storage;

// 重新导出公共依赖，供 Tauri 壳使用（壳层不再直接依赖这些 crate）
pub use chrono;
pub use serde_json;
pub use tokio;
pub use uuid;
