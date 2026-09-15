//! GPU 算子的 CPU 参考实现：**正确性基准**（GPU/CPU 一致性测试与数值对拍）。
//!
//! 后处理以硬件加速 GPU 为运行前提，**不提供 CPU 运行时回退**（详见
//! `ai-docs/ARCHITECTURE.md` §7）——故这里的实现只在测试与离线诊断中被调用，
//! 不要把它接成生产路径的回退分支。

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
