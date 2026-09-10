//! 派生结果算子（对标 moldflow-api data_transform 的「用户派生绘图」）：
//! 对已加载结果场做标量运算生成新场。CPU 参考实现为唯一事实源；
//! WGSL GPU 版本与一致性测试在 src-tauri 的 gpu_ops.rs（scalar_*）。

use crate::error::{KairosError, Result};

/// 归一化到 [0, 1]：(v - min) / (max - min)。极差为 0（常量场）时全 0。
pub fn normalize(values: &[f32]) -> Vec<f32> {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for &v in values {
        min = min.min(v);
        max = max.max(v);
    }
    let range = max - min;
    if range <= 0.0 {
        return vec![0.0; values.len()];
    }
    values.iter().map(|&v| (v - min) / range).collect()
}

/// 线性映射：v * scale + offset。
pub fn linear_map(values: &[f32], scale: f32, offset: f32) -> Vec<f32> {
    values.iter().map(|&v| v * scale + offset).collect()
}

/// 两场差值：a - b。长度不一致时报验证错误（不静默截断）。
pub fn field_difference(a: &[f32], b: &[f32]) -> Result<Vec<f32>> {
    if a.len() != b.len() {
        return Err(KairosError::validation(format!(
            "两场长度不一致：{} vs {}",
            a.len(),
            b.len()
        )));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x - y).collect())
}

/// 常数阈值掩码：v ≥ threshold → 1，否则 0。
pub fn threshold_mask(values: &[f32], threshold: f32) -> Vec<f32> {
    values
        .iter()
        .map(|&v| if v >= threshold { 1.0 } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_maps_min_max_to_zero_one() {
        assert_eq!(normalize(&[1.0, 2.0, 3.0]), vec![0.0, 0.5, 1.0]);
        // 常量场极差为 0：全 0 而非 NaN
        assert_eq!(normalize(&[7.0; 3]), vec![0.0; 3]);
        assert!(normalize(&[]).is_empty());
    }

    #[test]
    fn linear_map_applies_scale_and_offset() {
        assert_eq!(linear_map(&[1.0, 2.0], 2.0, -1.0), vec![1.0, 3.0]);
    }

    #[test]
    fn field_difference_rejects_length_mismatch() {
        assert!(field_difference(&[1.0], &[1.0, 2.0]).is_err());
        assert_eq!(
            field_difference(&[3.0, 5.0], &[1.0, 2.0]).unwrap(),
            vec![2.0, 3.0]
        );
    }

    #[test]
    fn threshold_mask_uses_inclusive_lower_bound() {
        assert_eq!(threshold_mask(&[0.4, 0.5, 0.6], 0.5), vec![0.0, 1.0, 1.0]);
    }
}
