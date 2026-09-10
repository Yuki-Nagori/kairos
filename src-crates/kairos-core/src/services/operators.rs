//! GPU 算子的 CPU 参考实现（T20）：GPU 不可用时的回退路径，
//! 也是 GPU/CPU 一致性测试的基准（唯一事实源，住 core 供各层复用）。

/// 矢量场模量：sqrt(vx² + vy² + vz²)。
pub fn vector_magnitude_cpu(vectors: &[[f32; 3]]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .collect()
}
