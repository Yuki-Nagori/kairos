//! `[f64; 3]` 向量原语：几何 / 网格 / 浇口等模块共用的最小数学。
//!
//! 这些函数此前在 6 个以上文件里各抄一份（写法略有出入、边界处理不一致），
//! 收敛到这里只保留一份；只放**无歧义的向量运算**，带产品口径的几何逻辑
//! （体素化、5-tet 分解、网格质量阈值、单位推断等）留在各自模块。

/// 逐分量相减。
pub fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// 叉积（右手系）。
pub fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// 点积。
pub fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// 欧氏距离平方（比较远近时避免开方）。
pub fn distance_sq(left: [f64; 3], right: [f64; 3]) -> f64 {
    let delta = sub(left, right);
    dot(delta, delta)
}

/// 欧氏距离。
pub fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    distance_sq(left, right).sqrt()
}

/// 单位化；零向量（或非有限）原样返回，由调用方按自己的口径兜底
/// ——几何里零法向的处理各不相同，这里不替调用方决定。
pub fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let norm = dot(vector, vector).sqrt();
    if norm > 0.0 && norm.is_finite() {
        [vector[0] / norm, vector[1] / norm, vector[2] / norm]
    } else {
        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_matches_hand_computation() {
        assert_eq!(sub([1.0, 2.0, 3.0], [0.5, 0.5, 0.5]), [0.5, 1.5, 2.5]);
        assert_eq!(dot([1.0, 2.0, 3.0], [4.0, -5.0, 6.0]), 12.0);
        // 右手系：x × y = z
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(distance([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]), 5.0);
        assert_eq!(distance_sq([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]), 25.0);
    }

    #[test]
    fn normalized_returns_unit_or_original_zero() {
        let unit = normalized([3.0, 0.0, 4.0]);
        assert!((unit[0] - 0.6).abs() < 1e-12);
        assert!((unit[2] - 0.8).abs() < 1e-12);
        // 零向量与非有限值原样返回：调用方各自兜底，不在这里造 NaN
        assert_eq!(normalized([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(
            normalized([f64::INFINITY, 0.0, 0.0]),
            [f64::INFINITY, 0.0, 0.0]
        );
    }
}
