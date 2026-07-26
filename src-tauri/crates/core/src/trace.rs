//! 消息状态追踪（推断式）
//!
//! 按 PRD §6.2 R7 技术说明：
//! v1 用 "队列级 Ready/Unacked 变化 + 消费者活跃度" 推断式状态。
//!
//! 状态机：
//! - Sent → 发送成功后立即标记
//! - Backlog → 发送后 1.3s 内队列有消费者但 ready 仍在涨
//! - Consumed → 队列 ready 下降 + 有活跃消费者
//! - Failed → 进入死信/被 reject
//!
//! 推断式无法 100% 精确（PRD 验收 ≥95% 即可），但成本低、覆盖大部分场景。

use crate::error::AppResult;
use crate::models::{MessageFeedItem, MessageStatus};
use crate::state::AppState;

impl AppState {
    /// 检查所有 sent/backlog 状态的消息，根据队列当前状态推断新状态
    pub async fn refresh_pending_status(&self) -> AppResult<()> {
        let active = self.active_connection.read().clone();
        if active.is_none() {
            return Ok(());
        }

        let feed = self.storage.list_feed(&crate::models::FeedFilter {
            queue: None,
            status: None,
            limit: Some(50),
        })?;

        for item in &feed {
            if item.status == MessageStatus::Sent || item.status == MessageStatus::Backlog {
                if let Ok(queue) = self
                    .rabbit_management()
                    .get_queue(&item.queue_name)
                    .await
                {
                    let new_status = infer_message_status(&queue, item);
                    if new_status != item.status {
                        self.storage.update_feed_status(&item.trace_id, new_status)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn infer_message_status(
    queue: &crate::rabbit::management::ManagementQueue,
    item: &MessageFeedItem,
) -> MessageStatus {
    if queue.consumers == 0 && queue.messages_ready > 0 {
        MessageStatus::Backlog
    } else if queue.messages_ready == 0 && queue.consumers > 0 {
        MessageStatus::Consumed
    } else {
        item.status
    }
}
