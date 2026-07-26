//! 全局 AppState

use crate::error::{AppError, AppResult};
use crate::models::Connection;
use crate::rabbit::{ConsumerManager, ManagementClient};
use crate::storage::Storage;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppState {
    pub storage: Storage,
    /// 当前活跃连接（含解密后的密码），用 RwLock 保护
    pub active_connection: RwLock<Option<ActiveConnection>>,
    /// 手动消费者管理器（消费者工作室）
    pub consumer_manager: ConsumerManager,
}

#[derive(Clone)]
pub struct ActiveConnection {
    pub connection: Connection,
    pub password: String,
    pub management: Arc<ManagementClient>,
}

impl AppState {
    pub fn new() -> AppResult<Self> {
        let app_data_dir = app_data_dir()?;
        let storage = Storage::open(app_data_dir)?;
        Ok(Self::with_storage(storage))
    }

    /// 用指定 Storage 构造（主要用于测试）
    pub fn with_storage(storage: Storage) -> Self {
        Self {
            storage,
            active_connection: RwLock::new(None),
            consumer_manager: ConsumerManager::new(),
        }
    }

    /// 设置当前活跃连接，并持久化 last_active_id
    pub fn set_active(&self, conn: Connection, password: String) {
        let management = Arc::new(ManagementClient::new(
            &conn.management_scheme,
            &conn.host,
            conn.management_port,
            &conn.vhost,
            &conn.username,
            &password,
        ));
        let active = ActiveConnection {
            connection: conn.clone(),
            password,
            management,
        };
        *self.active_connection.write() = Some(active);
        let _ = self.storage.save_last_active_id(&conn.id);
    }

    pub fn clear_active(&self) {
        *self.active_connection.write() = None;
        let _ = self.storage.clear_last_active_id();
    }

    /// 启动时尝试恢复上次活跃连接（不真正测试连通性，仅设置内存状态）
    pub fn restore_last_active(&self) -> AppResult<Option<Connection>> {
        let Some(id) = self.storage.load_last_active_id()? else {
            return Ok(None);
        };
        let conn = self.storage.get_connection(&id)?;
        let password = self.storage.load_password(&id)?;
        self.set_active(conn.clone(), password);
        Ok(Some(conn))
    }

    pub fn rabbit_management(&self) -> Arc<ManagementClient> {
        self.active_connection
            .read()
            .as_ref()
            .expect("rabbit_management called without active connection")
            .management
            .clone()
    }

    /// 获取活跃连接信息（不返回密码）
    pub fn get_active(&self) -> Option<Connection> {
        self.active_connection.read().as_ref().map(|a| {
            let mut c = a.connection.clone();
            c.password = String::new();
            c
        })
    }

    /// 拿当前活跃连接的 AMQP URL（用于 publisher）
    ///
    /// 注意：vhost 在 AMQP URL 中需要 percent-encode。
    /// RabbitMQ 默认 vhost "/" 必须编码为 %2F，否则会被当成空字符串导致 "vhost not found"。
    pub fn amqp_url(&self) -> AppResult<String> {
        let guard = self.active_connection.read();
        let active = guard.as_ref().ok_or(AppError::NotConnected)?;
        let vhost_encoded = percent_encode(&active.connection.vhost);
        Ok(format!(
            "amqp://{}:{}@{}:{}/{}",
            percent_encode(&active.connection.username),
            percent_encode(&active.password),
            active.connection.host,
            active.connection.amqp_port,
            vhost_encoded
        ))
    }
}

/// 用户数据目录：Windows 为 %APPDATA%\mqdesk
fn app_data_dir() -> AppResult<PathBuf> {
    if let Some(dir) = dirs_next::data_dir() {
        Ok(dir.join("mqdesk"))
    } else {
        Err(AppError::Storage("无法确定用户数据目录".to_string()))
    }
}

mod dirs_next {
    use std::path::PathBuf;

    pub fn data_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA").map(PathBuf::from)
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        }
    }
}

/// 简单的 percent-encoding，用于 AMQP URL 中的用户名/密码
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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
