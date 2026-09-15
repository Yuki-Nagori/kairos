//! 时间戳工具。
//!
//! 时间戳只在编排层与适配层取，不要散落到算法内部——算法接收时间参数才可测试
//! （`services::jobs` 的 `now_ms: u64` 形参就是这个形态，保持它）。

use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 Unix 毫秒时间戳。
///
/// 系统时钟早于 epoch（人为调表、容器时钟未同步等）时返回 0 而不是 panic：
/// 时间戳只用于排序与展示，退化成一个固定值不影响功能。
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ms_is_after_epoch() {
        assert!(now_ms() > 0);
    }

    #[test]
    fn now_ms_is_monotonic_across_calls() {
        let earlier = now_ms();
        let later = now_ms();
        assert!(later >= earlier);
    }
}
