//! 本地存储：sled 数据库 + 本地 AES-256-GCM 密码加密

use crate::error::{AppError, AppResult};
use crate::models::Connection;
use chrono::Utc;
use sled::{Db, Tree};
use std::path::PathBuf;

const CONNECTIONS_TREE: &str = "connections";
const PASSWORDS_TREE: &str = "passwords";
const FEED_TREE: &str = "message_feed";
const ALERT_RULES_TREE: &str = "alert_rules";
const ALERT_RECORDS_TREE: &str = "alert_records";
const AUDIT_LOGS_TREE: &str = "audit_logs";
const LAST_ACTIVE_KEY: &str = "last_active_id";

/// 本地存储管理器
pub struct Storage {
    db: Db,
    connections: Tree,
    passwords: Tree,
    feed: Tree,
    alert_rules: Tree,
    alert_records: Tree,
    audit_logs: Tree,
}

impl Storage {
    /// 在用户数据目录下打开/创建 mqdesk.db
    pub fn open(app_data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| AppError::Storage(e.to_string()))?;
        let db_path = app_data_dir.join("mqdesk.db");
        let db = sled::open(db_path)?;
        let connections = db.open_tree(CONNECTIONS_TREE)?;
        let passwords = db.open_tree(PASSWORDS_TREE)?;
        let feed = db.open_tree(FEED_TREE)?;
        let alert_rules = db.open_tree(ALERT_RULES_TREE)?;
        let alert_records = db.open_tree(ALERT_RECORDS_TREE)?;
        let audit_logs = db.open_tree(AUDIT_LOGS_TREE)?;
        Ok(Self { db, connections, passwords, feed, alert_rules, alert_records, audit_logs })
    }

    // === 最近活跃连接 ===

    pub fn save_last_active_id(&self, id: &str) -> AppResult<()> {
        self.db.insert(LAST_ACTIVE_KEY, id.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_last_active_id(&self) -> AppResult<Option<String>> {
        match self.db.get(LAST_ACTIVE_KEY)? {
            Some(value) => {
                let id = String::from_utf8(value.to_vec())
                    .map_err(|_| AppError::Storage("last_active_id 不是有效 UTF-8".to_string()))?;
                Ok(Some(id))
            }
            None => Ok(None),
        }
    }

    pub fn clear_last_active_id(&self) -> AppResult<()> {
        self.db.remove(LAST_ACTIVE_KEY)?;
        self.db.flush()?;
        Ok(())
    }

    // === 连接 ===

    pub fn list_connections(&self) -> AppResult<Vec<Connection>> {
        let mut items = Vec::new();
        for item in self.connections.iter() {
            let (_, value) = item?;
            let conn: Connection = serde_json::from_slice(&value)?;
            items.push(conn);
        }
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(items)
    }

    pub fn get_connection(&self, id: &str) -> AppResult<Connection> {
        let key = id.as_bytes();
        let value = self
            .connections
            .get(key)?
            .ok_or_else(|| AppError::ConnectionNotFound(id.to_string()))?;
        let conn: Connection = serde_json::from_slice(&value)?;
        Ok(conn)
    }

    pub fn upsert_connection(&self, conn: Connection) -> AppResult<Connection> {
        let key = conn.id.as_bytes();
        let value = serde_json::to_vec(&conn)?;
        self.connections.insert(key, value)?;
        self.db.flush()?;
        Ok(conn)
    }

    pub fn delete_connection(&self, id: &str) -> AppResult<()> {
        self.connections.remove(id.as_bytes())?;
        self.db.flush()?;
        // 同步删除本地加密存储的密码
        self.passwords.remove(id.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    // === 密码加密（本地 AES-256-GCM） ===

    pub fn save_password(&self, id: &str, password: &str) -> AppResult<()> {
        let encrypted = crate::crypto::encrypt(password)?;
        self.passwords.insert(id.as_bytes(), encrypted.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_password(&self, id: &str) -> AppResult<String> {
        let value = self
            .passwords
            .get(id.as_bytes())?
            .ok_or_else(|| AppError::Crypto(format!("未找到连接 {id} 的密码")))?;
        let encrypted = std::str::from_utf8(&value)
            .map_err(|_| AppError::Crypto("密码密文不是有效 UTF-8".to_string()))?;
        crate::crypto::decrypt(encrypted)
    }

    // === 消息流 ===

    pub fn append_feed(&self, item: &crate::models::MessageFeedItem) -> AppResult<()> {
        let key = format!("{}|{}", item.time, item.trace_id);
        let value = serde_json::to_vec(item)?;
        self.feed.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn list_feed(
        &self,
        filter: &crate::models::FeedFilter,
    ) -> AppResult<Vec<crate::models::MessageFeedItem>> {
        let limit = filter.limit.unwrap_or(200);
        let mut items = Vec::new();
        // sled 默认按 key 升序，我们要按时间倒序，所以反向遍历
        for item in self.feed.iter().rev() {
            let (_, value) = item?;
            let feed_item: crate::models::MessageFeedItem = serde_json::from_slice(&value)?;

            if let Some(ref q) = filter.queue {
                if feed_item.queue_name != *q {
                    continue;
                }
            }
            if let Some(ref s) = filter.status {
                if feed_item.status.value() != *s {
                    continue;
                }
            }
            items.push(feed_item);
            if items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }

    pub fn get_feed(&self, trace_id: &str) -> AppResult<crate::models::MessageFeedItem> {
        for item in self.feed.iter() {
            let (_, value) = item?;
            let feed_item: crate::models::MessageFeedItem = serde_json::from_slice(&value)?;
            if feed_item.trace_id == trace_id {
                return Ok(feed_item);
            }
        }
        Err(crate::error::AppError::Storage(format!("未找到消息 {trace_id}")))
    }

    pub fn update_feed_status(&self, trace_id: &str, new_status: crate::models::MessageStatus) -> AppResult<()> {
        for item in self.feed.iter() {
            let (key, value) = item?;
            let mut feed_item: crate::models::MessageFeedItem = serde_json::from_slice(&value)?;
            if feed_item.trace_id == trace_id {
                feed_item.status = new_status;
                let new_value = serde_json::to_vec(&feed_item)?;
                self.feed.insert(&key, new_value)?;
                self.db.flush()?;
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn delete_feed(&self, trace_id: &str) -> AppResult<()> {
        let keys_to_remove: Vec<sled::IVec> = self
            .feed
            .iter()
            .filter_map(|item| {
                let (key, value) = item.ok()?;
                let feed_item: crate::models::MessageFeedItem = serde_json::from_slice(&value).ok()?;
                if feed_item.trace_id == trace_id {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        for key in keys_to_remove {
            self.feed.remove(key)?;
        }
        self.db.flush()?;
        Ok(())
    }

    // === 告警规则 ===

    fn alert_rule_key(queue_name: &str, vhost: &str, metric: &str) -> String {
        format!("{}|{}|{}", vhost, queue_name, metric)
    }

    pub fn set_alert_rule(&self, rule: &crate::models::QueueAlertRule) -> AppResult<()> {
        let key = Self::alert_rule_key(&rule.queue_name, &rule.vhost, &rule.metric);
        let value = serde_json::to_vec(rule)?;
        self.alert_rules.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn list_alert_rules(
        &self,
        queue_name: Option<&str>,
        vhost: Option<&str>,
    ) -> AppResult<Vec<crate::models::QueueAlertRule>> {
        let mut rules = Vec::new();
        for item in self.alert_rules.iter() {
            let (_, value) = item?;
            let rule: crate::models::QueueAlertRule = serde_json::from_slice(&value)?;
            if let Some(q) = queue_name {
                if rule.queue_name != q {
                    continue;
                }
            }
            if let Some(v) = vhost {
                if rule.vhost != v {
                    continue;
                }
            }
            rules.push(rule);
        }
        Ok(rules)
    }

    pub fn delete_alert_rule(
        &self,
        queue_name: &str,
        vhost: &str,
        metric: &str,
    ) -> AppResult<()> {
        let key = Self::alert_rule_key(queue_name, vhost, metric);
        self.alert_rules.remove(key.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    // === 告警记录 ===

    pub fn append_alert_record(
        &self,
        record: &crate::models::QueueAlertRecord,
    ) -> AppResult<()> {
        let key = format!("{}|{}", record.triggered_at, record.id);
        let value = serde_json::to_vec(record)?;
        self.alert_records.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn list_alert_records(
        &self,
        queue_name: Option<&str>,
        vhost: Option<&str>,
        limit: usize,
    ) -> AppResult<Vec<crate::models::QueueAlertRecord>> {
        let mut records = Vec::new();
        for item in self.alert_records.iter().rev() {
            let (_, value) = item?;
            let record: crate::models::QueueAlertRecord = serde_json::from_slice(&value)?;
            if let Some(q) = queue_name {
                if record.queue_name != q {
                    continue;
                }
            }
            if let Some(v) = vhost {
                if record.vhost != v {
                    continue;
                }
            }
            records.push(record);
            if records.len() >= limit {
                break;
            }
        }
        Ok(records)
    }

    pub fn resolve_alert_record(
        &self,
        queue_name: &str,
        vhost: &str,
        metric: &str,
        resolved_at: &str,
    ) -> AppResult<()> {
        let target_key = Self::alert_rule_key(queue_name, vhost, metric);
        for item in self.alert_records.iter().rev() {
            let (key, value) = item?;
            let mut record: crate::models::QueueAlertRecord = serde_json::from_slice(&value)?;
            let record_key = Self::alert_rule_key(&record.queue_name, &record.vhost, &record.metric);
            if record_key == target_key && record.resolved_at.is_none() {
                record.resolved_at = Some(resolved_at.to_string());
                let new_value = serde_json::to_vec(&record)?;
                self.alert_records.insert(&key, new_value)?;
                self.db.flush()?;
                return Ok(());
            }
        }
        Ok(())
    }

    // === 审计日志 ===

    pub fn append_audit_log(&self, log: &crate::models::QueueAuditLog) -> AppResult<()> {
        let key = format!("{}|{}", log.timestamp, log.id);
        let value = serde_json::to_vec(log)?;
        self.audit_logs.insert(key.as_bytes(), value)?;
        // 仅保留最近 1000 条
        if self.audit_logs.len() > 1000 {
            if let Some(oldest) = self.audit_logs.first()? {
                self.audit_logs.remove(oldest.0)?;
            }
        }
        self.db.flush()?;
        Ok(())
    }

    pub fn list_audit_logs(
        &self,
        queue_name: Option<&str>,
        vhost: Option<&str>,
        limit: usize,
    ) -> AppResult<Vec<crate::models::QueueAuditLog>> {
        let mut logs = Vec::new();
        for item in self.audit_logs.iter().rev() {
            let (_, value) = item?;
            let log: crate::models::QueueAuditLog = serde_json::from_slice(&value)?;
            if let Some(q) = queue_name {
                if log.target_queue != q {
                    continue;
                }
            }
            if let Some(v) = vhost {
                if log.vhost != v {
                    continue;
                }
            }
            logs.push(log);
            if logs.len() >= limit {
                break;
            }
        }
        Ok(logs)
    }
}

/// 生成时间戳字符串（用于 created_at/updated_at）
pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
