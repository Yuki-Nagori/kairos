//! GPU 算子的 CPU 参考实现（T20）：GPU 不可用时的回退路径，
//! 也是 GPU/CPU 一致性测试的基准（唯一事实源，住 core 供各层复用）。

/// 矢量场模量：sqrt(vx² + vy² + vz²)。
pub fn vector_magnitude_cpu(vectors: &[[f32; 3]]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magnitudes_match_pythagoras() {
        assert_eq!(vector_magnitude_cpu(&[[3.0, 4.0, 0.0]]), [5.0]);
        assert_eq!(vector_magnitude_cpu(&[[1.0, 2.0, 2.0]]), [3.0]);
        assert_eq!(vector_magnitude_cpu(&[[0.0, 0.0, 0.0]]), [0.0]);
        assert!(vector_magnitude_cpu(&[]).is_empty());
    }
}
