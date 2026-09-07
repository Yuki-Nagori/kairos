//! GPU 加速后处理算子（T20）：wgpu compute + CPU 参考实现。
//! 算子清单（v1）：矢量模量（速度场后处理）。归一化 / LOD / 切片按同一模式扩展。

use serde::Serialize;

use kairos_core::error::KairosError;
use wgpu::util::DeviceExt;

const VECTOR_MAGNITUDE_SHADER: &str = r#"
struct Vectors {
    data: array<vec4<f32>>,
}
@group(0) @binding(0) var<storage, read> input: Vectors;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let index = gid.x;
    if (index >= arrayLength(&input.data)) {
        return;
    }
    output[index] = length(input.data[index].xyz);
}
"#;

const WORKGROUP_SIZE: usize = 64;

/// CPU 参考实现：矢量模量（GPU 不可用时的回退路径，也是一致性测试的基准）。
pub fn vector_magnitude_cpu(vectors: &[[f32; 3]]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .collect()
}

fn create_device() -> Result<(wgpu::Device, wgpu::Queue), KairosError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| KairosError::internal("无可用 GPU 适配器（CPU 回退路径可完成相同计算）。"))?;
    let adapter_clone_info = adapter.get_info();
    let _ = adapter_clone_info;
    let future = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("kairos-compute"),
            ..Default::default()
        },
        None,
    );
    let device_queue = pollster::block_on(future)
        .map_err(|e| KairosError::internal(format!("GPU 设备创建失败：{e}")))?;
    Ok(device_queue)
}

/// GPU 算子：矢量模量。输入按 vec4 对齐；在无 GPU 环境返回内部错误，由调用方回退 CPU。
pub fn vector_magnitude_gpu(vectors: &[[f32; 3]]) -> Result<Vec<f32>, KairosError> {
    if vectors.is_empty() {
        return Ok(Vec::new());
    }
    let (device, queue) = create_device()?;

    let mut flat: Vec<f32> = Vec::with_capacity(vectors.len() * 4);
    for vector in vectors {
        flat.extend_from_slice(&[vector[0], vector[1], vector[2], 0.0]);
    }
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vector_magnitude_input"),
        contents: bytemuck::cast_slice(&flat),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vector_magnitude_output"),
        contents: vec![0u8; vectors.len() * 4].as_slice(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vector_magnitude"),
        source: wgpu::ShaderSource::Wgsl(VECTOR_MAGNITUDE_SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("vector_magnitude"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("vector_magnitude_bind"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // MAP_READ 暂存缓冲：GPU → CPU 回读必须经由拷贝。
    let read_back_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vector_magnitude_read_back"),
        size: (vectors.len() * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let workgroups = vectors.len().div_ceil(WORKGROUP_SIZE) as u32;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vector_magnitude"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("vector_magnitude_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    let byte_size = (vectors.len() * 4) as u64;
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &read_back_buffer, 0, byte_size);
    queue.submit(Some(encoder.finish()));

    let slice = read_back_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|e| KairosError::internal(format!("GPU 回读信号丢失：{e}")))?
        .map_err(|e| KairosError::internal(format!("GPU 映射失败：{e}")))?;

    let mapped = slice.get_mapped_range();
    let result: Vec<f32> = mapped
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("4 字节")))
        .collect();
    drop(mapped);
    Ok(result)
}

/// GPU 算子元数据（诊断页展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuOperatorInfo {
    pub name: String,
    pub description: String,
}

pub fn operator_catalog() -> Vec<GpuOperatorInfo> {
    vec![GpuOperatorInfo {
        name: "vector_magnitude".into(),
        description: "矢量场模量（速度场后处理）".into(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deterministic_vectors(count: usize) -> Vec<[f32; 3]> {
        // 确定性伪随机（LCG），保证测试可复现。
        let mut state: u32 = 0x1234_5678;
        let mut vectors = Vec::with_capacity(count);
        for _ in 0..count {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = (state >> 8) as f32 / 16777216.0 * 20.0 - 10.0;
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = (state >> 8) as f32 / 16777216.0 * 20.0 - 10.0;
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let z = (state >> 8) as f32 / 16777216.0 * 20.0 - 10.0;
            vectors.push([x, y, z]);
        }
        vectors
    }

    #[test]
    fn cpu_reference_matches_expected() {
        let vectors = vec![[3.0, 4.0, 0.0], [1.0, 0.0, 0.0]];
        let magnitudes = vector_magnitude_cpu(&vectors);
        assert_eq!(magnitudes, vec![5.0, 1.0]);
    }

    #[test]
    fn cpu_reference_handles_large_deterministic_input() {
        let vectors = deterministic_vectors(1024);
        let magnitudes = vector_magnitude_cpu(&vectors);
        assert_eq!(magnitudes.len(), 1024);
        assert!(magnitudes.iter().all(|m| m.is_finite()));
    }

    #[test]
    fn operator_catalog_is_non_empty() {
        assert!(!operator_catalog().is_empty());
    }

    #[test]
    fn gpu_magnitudes_match_cpu_reference() {
        let vectors = deterministic_vectors(1024);
        let gpu = vector_magnitude_gpu(&vectors).expect("GPU 管线应在本机可用");
        let cpu = vector_magnitude_cpu(&vectors);
        assert_eq!(gpu.len(), cpu.len());
        for (gpu_value, cpu_value) in gpu.iter().zip(cpu.iter()) {
            assert!(
                (gpu_value - cpu_value).abs() < 1e-4,
                "GPU {gpu_value} vs CPU {cpu_value}"
            );
        }
    }
}
