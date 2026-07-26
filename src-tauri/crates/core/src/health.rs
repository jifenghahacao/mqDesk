//! 队列健康度四态判定
//!
//! 规则（按 PRD §6.2 R6）：
//! - 🟢 正常（Ok）：有消费者且 Ready 在低位
//! - 🟡 堆积预警（Warn）：Ready > 阈值（默认 1000）且持续增长
//! - 🔴 无人消费（Danger）：消费者数 = 0 且 Ready > 0
//! - ⚪ 空闲（Idle）：Ready = 0
//!
//! 简化版（不持续监控增长趋势，按瞬时数据判定）：
//! - consumers == 0 且 ready > 0 → Danger
//! - ready == 0 → Idle
//! - ready > 阈值 → Warn
//! - 其他 → Ok

use crate::models::HealthStatus;

const READY_WARN_THRESHOLD: u64 = 1000;

pub fn judge_health(
    ready: u64,
    consumers: u64,
    _incoming_rate: f64,
    _outgoing_rate: f64,
) -> HealthStatus {
    if ready == 0 {
        HealthStatus::Idle
    } else if consumers == 0 {
        HealthStatus::Danger
    } else if ready > READY_WARN_THRESHOLD {
        HealthStatus::Warn
    } else {
        HealthStatus::Ok
    }
}

pub fn health_summary(status: &HealthStatus) -> String {
    match status {
        HealthStatus::Ok => "消费者正常工作中，收发速率基本平衡。".to_string(),
        HealthStatus::Warn => "待消费数偏高且持续增长，建议检查消费端速率。".to_string(),
        HealthStatus::Danger => "当前无消费者在取消息，消息会一直堆积，请确认消费程序是否在线。".to_string(),
        HealthStatus::Idle => "队列为空，暂无待处理消息。".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_when_ready_zero() {
        assert_eq!(
            judge_health(0, 0, 0.0, 0.0),
            HealthStatus::Idle
        );
        assert_eq!(
            judge_health(0, 5, 10.0, 10.0),
            HealthStatus::Idle
        );
    }

    #[test]
    fn danger_when_no_consumers_but_ready() {
        assert_eq!(
            judge_health(100, 0, 10.0, 0.0),
            HealthStatus::Danger
        );
    }

    #[test]
    fn warn_when_ready_over_threshold() {
        assert_eq!(
            judge_health(1500, 3, 50.0, 30.0),
            HealthStatus::Warn
        );
    }

    #[test]
    fn ok_when_consumers_and_low_ready() {
        assert_eq!(
            judge_health(50, 2, 30.0, 28.0),
            HealthStatus::Ok
        );
    }
}
