//! Management API 调用限流器
//!
//! 使用令牌桶算法限制每秒对 RabbitMQ Management API 的调用次数，
//! 避免 MQDesk 自身的高频请求压垮服务端。

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// 全局 Management API 限流器
#[derive(Debug, Clone)]
pub struct ManagementRateLimiter {
    inner: Arc<Mutex<Inner>>,
    max_per_second: u32,
}

#[derive(Debug)]
struct Inner {
    /// 当前可用令牌数
    tokens: f64,
    /// 上次更新时间
    last_updated: Instant,
}

impl ManagementRateLimiter {
    /// 创建一个新的限流器
    ///
    /// # Arguments
    /// * `max_per_second` - 每秒允许的最多请求数
    pub fn new(max_per_second: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                tokens: max_per_second as f64,
                last_updated: Instant::now(),
            })),
            max_per_second,
        }
    }

    /// 获取一个令牌，如果当前没有可用令牌则等待
    pub async fn acquire(&self) {
        loop {
            let mut inner = self.inner.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(inner.last_updated).as_secs_f64();

            // 补充令牌，但不超过桶容量
            inner.tokens = (inner.tokens + elapsed * self.max_per_second as f64)
                .min(self.max_per_second as f64);
            inner.last_updated = now;

            if inner.tokens >= 1.0 {
                inner.tokens -= 1.0;
                return;
            }

            // 计算需要等待的时间
            let wait_seconds = (1.0 - inner.tokens) / self.max_per_second as f64;
            let wait = Duration::from_secs_f64(wait_seconds.max(0.001));
            drop(inner);
            tokio::time::sleep(wait).await;
        }
    }
}

impl Default for ManagementRateLimiter {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_immediately_when_tokens_available() {
        let limiter = ManagementRateLimiter::new(10);
        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_rate_limiting_basic() {
        let limiter = ManagementRateLimiter::new(10);
        let start = Instant::now();

        // 前 10 次应该立即通过
        for _ in 0..10 {
            limiter.acquire().await;
        }
        let first_batch_elapsed = start.elapsed();
        assert!(first_batch_elapsed < Duration::from_millis(100));

        // 第 11 次应该需要等待约 100ms
        limiter.acquire().await;
        let total_elapsed = start.elapsed();
        assert!(total_elapsed >= Duration::from_millis(80));
    }

    #[tokio::test]
    async fn test_burst_cap() {
        let limiter = ManagementRateLimiter::new(5);

        // 先消耗一次
        limiter.acquire().await;

        // 等待 2 秒，理论上最多补充到 5 个令牌
        tokio::time::sleep(Duration::from_secs(2)).await;

        let start = Instant::now();
        // 5 次应该能立即通过（桶容量上限）
        for _ in 0..5 {
            limiter.acquire().await;
        }
        assert!(start.elapsed() < Duration::from_millis(100));

        // 第 6 次需要等待
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(150));
    }
}
