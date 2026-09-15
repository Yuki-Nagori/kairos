//! 浮点比较与聚合工具。
//!
//! 排序统一用 `f64::total_cmp`：它给出全序（NaN 排在末尾），既不 panic，也不像
//! `partial_cmp(..).unwrap_or(Ordering::Equal)` 那样把 NaN 当相等——后者会让排序
//! 结果依赖输入顺序，同样的数据换个顺序得出不同的表。
//!
//! 这里**不**统一 epsilon 常量：现存各处的 `1e-9` / `1e-12` 各有量纲与来历，
//! 机械合并会改变数值行为。[`safe_ratio`] 的 epsilon 由调用方显式传入，保持每个
//! 调用点的物理含义可见。

/// 轴对齐包围盒：用 [`Aabb::extend_point`] 逐步累积，调用方不必先收集所有点。
///
/// 初值取正负无穷而非 `f64::MAX` / `f64::MIN`：判空才不必依赖减法溢出。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    min: [f64; 3],
    max: [f64; 3],
}

impl Aabb {
    /// 空盒：任何点都能扩展它。
    pub const fn empty() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }

    /// 纳入一个点。
    pub fn extend_point(&mut self, point: [f64; 3]) {
        for (min, value) in self.min.iter_mut().zip(point) {
            *min = min.min(value);
        }
        for (max, value) in self.max.iter_mut().zip(point) {
            *max = max.max(value);
        }
    }

    /// 纳入一组点。
    pub fn extend_points(&mut self, points: &[[f64; 3]]) {
        for point in points {
            self.extend_point(*point);
        }
    }

    /// 是否尚未纳入任何点。三轴始终同步更新，故看任一轴即可。
    pub fn is_empty(&self) -> bool {
        self.min[0] > self.max[0]
    }

    /// 各轴下界与上界；空盒返回 `None`。
    pub fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        if self.is_empty() {
            return None;
        }
        Some((self.min, self.max))
    }
}

/// 升序排序（NaN 排在末尾）。
pub fn sort_asc(values: &mut [f64]) {
    values.sort_by(f64::total_cmp);
}

/// 降序排序（`total_cmp` 的逆序，NaN 被排到最前）。
pub fn sort_desc(values: &mut [f64]) {
    values.sort_by(|left, right| right.total_cmp(left));
}

/// 最小与最大值；空切片返回 `None`。
pub fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &value in values {
        min = min.min(value);
        max = max.max(value);
    }
    if min > max {
        return None;
    }
    Some((min, max))
}

/// 安全比例：分母的绝对值不超过 `epsilon` 时返回 0，避免 inf / NaN 污染后续统计。
pub fn safe_ratio(numerator: f64, denominator: f64, epsilon: f64) -> f64 {
    if denominator.abs() <= epsilon {
        return 0.0;
    }
    numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_aabb_reports_no_bounds() {
        let boxed = Aabb::empty();
        assert!(boxed.is_empty());
        assert_eq!(boxed.bounds(), None);
    }

    #[test]
    fn extends_over_every_axis() {
        let mut boxed = Aabb::empty();
        boxed.extend_point([1.0, -2.0, 3.0]);
        boxed.extend_point([-1.0, 5.0, 0.0]);
        assert!(!boxed.is_empty());
        assert_eq!(boxed.bounds(), Some(([-1.0, -2.0, 0.0], [1.0, 5.0, 3.0])));
    }

    #[test]
    fn extends_over_point_slices() {
        let mut boxed = Aabb::empty();
        boxed.extend_points(&[[0.0, 0.0, 0.0], [2.0, 2.0, 2.0]]);
        assert_eq!(boxed.bounds(), Some(([0.0; 3], [2.0; 3])));
    }

    #[test]
    fn extending_with_no_points_keeps_it_empty() {
        let mut boxed = Aabb::empty();
        boxed.extend_points(&[]);
        assert!(boxed.is_empty());
    }

    #[test]
    fn sort_asc_orders_values() {
        let mut values = [3.0, 1.0, 2.0];
        sort_asc(&mut values);
        assert_eq!(values, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn sort_desc_orders_values() {
        let mut values = [3.0, 1.0, 2.0];
        sort_desc(&mut values);
        assert_eq!(values, [3.0, 2.0, 1.0]);
    }

    #[test]
    fn sort_places_nan_at_the_end_for_ascending() {
        let mut values = [2.0, f64::NAN, 1.0];
        sort_asc(&mut values);
        assert_eq!(values[0], 1.0);
        assert_eq!(values[1], 2.0);
        assert!(values[2].is_nan());
    }

    #[test]
    fn min_max_reports_both_ends() {
        assert_eq!(min_max(&[3.0, 1.0, 2.0]), Some((1.0, 3.0)));
        assert_eq!(min_max(&[-5.0]), Some((-5.0, -5.0)));
    }

    #[test]
    fn min_max_is_none_for_empty_slice() {
        assert_eq!(min_max(&[]), None);
    }

    #[test]
    fn safe_ratio_divides_when_denominator_is_meaningful() {
        assert_eq!(safe_ratio(1.0, 4.0, 1e-9), 0.25);
    }

    #[test]
    fn safe_ratio_returns_zero_for_degenerate_denominator() {
        assert_eq!(safe_ratio(1.0, 0.0, 1e-9), 0.0);
        assert_eq!(safe_ratio(1.0, 1e-12, 1e-9), 0.0);
        assert_eq!(safe_ratio(1.0, -1e-12, 1e-9), 0.0);
    }
}
