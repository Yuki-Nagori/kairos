//! GPU 加速后处理算子：wgpu compute + CPU 参考实现。
//! 算子清单（v1）：矢量模量（速度场后处理）。归一化 / LOD / 切片按同一模式扩展。

use serde::Serialize;

use kairos_core::error::KairosError;
use wgpu::util::DeviceExt;

// 着色器源码外置在 src-tauri/shaders/（编译期 include_str! 内联），算子目录化铺路。
const VECTOR_MAGNITUDE_SHADER: &str = include_str!("../../shaders/vector_magnitude.wgsl");

const WORKGROUP_SIZE: usize = 64;

#[cfg(test)]
use kairos_core::services::operators::vector_magnitude_cpu;

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
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect();
    drop(mapped);
    Ok(result)
}

// ─── 派生标量算子：线性映射 / 阈值掩码 / 两场差值 ───

const SCALAR_LINEAR_SHADER: &str = include_str!("../../shaders/scalar_linear.wgsl");
const SCALAR_THRESHOLD_SHADER: &str = include_str!("../../shaders/scalar_threshold.wgsl");
const SCALAR_DIFFERENCE_SHADER: &str = include_str!("../../shaders/scalar_difference.wgsl");

/// 通用标量 compute 管线：1~2 个 f32 输入缓冲 + 可选参数缓冲（16 字节对齐
/// 的 uniform 形态打包为 4×f32），输出 len 个 f32 回读。
fn run_scalar_pipeline(
    label: &str,
    shader: &str,
    inputs: &[&[f32]],
    params: &[f32; 4],
    len: usize,
) -> Result<Vec<f32>, KairosError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let (device, queue) = create_device()?;

    let make_buffer = |name: &str, data: &[u8], read_back: bool| {
        let mut usage = wgpu::BufferUsages::STORAGE;
        if read_back {
            usage |= wgpu::BufferUsages::COPY_SRC;
        }
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(name),
            contents: data,
            usage,
        })
    };

    let input_a = make_buffer(
        &format!("{label}_in_a"),
        bytemuck::cast_slice(inputs[0]),
        false,
    );
    let input_b = inputs
        .get(1)
        .map(|data| make_buffer(&format!("{label}_in_b"), bytemuck::cast_slice(data), false));
    // 参数缓冲：非空时创建（差值算子等无参着色器跳过，避免无用分配）
    let params_buffer = if params.is_empty() {
        None
    } else {
        Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{label}_params")),
                contents: bytemuck::cast_slice(params),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        )
    };
    let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label}_out")),
        contents: vec![0u8; len * 4].as_slice(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // 两种绑定布局：带参数（线性/阈值：in@0, params@1, out@2）
    // 与双输入（差值：a@0, b@1, out@2，无参数缓冲）。
    let entries: Vec<wgpu::BindGroupEntry> = match &input_b {
        None => {
            let pb = params_buffer
                .as_ref()
                .ok_or_else(|| KairosError::internal("带参着色器缺少参数缓冲"))?;
            vec![
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pb.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ]
        }
        Some(b) => vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: b.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    };
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{label}_bind")),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });

    let read_back_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{label}_read_back")),
        size: (len * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let workgroups = len.div_ceil(WORKGROUP_SIZE) as u32;
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&format!("{label}_pass")),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    let byte_size = (len * 4) as u64;
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
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect();
    drop(mapped);
    Ok(result)
}

/// GPU 算子：线性映射（归一化由 CPU 预计算 min/max 后以 scale/offset 表达）。
pub fn scalar_linear_gpu(values: &[f32], scale: f32, offset: f32) -> Result<Vec<f32>, KairosError> {
    run_scalar_pipeline(
        "scalar_linear",
        SCALAR_LINEAR_SHADER,
        &[values],
        &[scale, offset, 0.0, 0.0],
        values.len(),
    )
}

/// GPU 算子：阈值掩码（v ≥ threshold → 1）。
pub fn scalar_threshold_gpu(values: &[f32], threshold: f32) -> Result<Vec<f32>, KairosError> {
    run_scalar_pipeline(
        "scalar_threshold",
        SCALAR_THRESHOLD_SHADER,
        &[values],
        &[threshold, 0.0, 0.0, 0.0],
        values.len(),
    )
}

/// GPU 算子：两场差值 a - b（长度由调用方先校验）。
pub fn scalar_difference_gpu(a: &[f32], b: &[f32]) -> Result<Vec<f32>, KairosError> {
    run_scalar_pipeline(
        "scalar_difference",
        SCALAR_DIFFERENCE_SHADER,
        &[a, b],
        &[0.0, 0.0, 0.0, 0.0],
        a.len(),
    )
}

/// GPU 算子元数据（诊断页展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuOperatorInfo {
    pub name: String,
    pub description: String,
}

pub fn operator_catalog() -> Vec<GpuOperatorInfo> {
    vec![
        GpuOperatorInfo {
            name: "vector_magnitude".into(),
            description: "矢量场模量（速度场后处理）".into(),
        },
        GpuOperatorInfo {
            name: "derive_linear".into(),
            description: "派生：线性映射（含归一化）".into(),
        },
        GpuOperatorInfo {
            name: "derive_threshold".into(),
            description: "派生：常数阈值掩码".into(),
        },
        GpuOperatorInfo {
            name: "derive_difference".into(),
            description: "派生：两场差值".into(),
        },
    ]
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
        assert!(
            operator_catalog()
                .iter()
                .any(|op| op.name == "derive_linear")
        );
    }

    fn deterministic_scalars(count: usize) -> Vec<f32> {
        let mut state: u32 = 0xABCD_1234;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            values.push((state >> 8) as f32 / 16777216.0 * 10.0);
        }
        values
    }

    #[test]
    fn scalar_linear_gpu_matches_cpu() {
        let values = deterministic_scalars(1000);
        let gpu = scalar_linear_gpu(&values, 0.5, -2.0).expect("GPU 线性映射");
        let reference: Vec<f32> = values.iter().map(|&v| v * 0.5 - 2.0).collect();
        for (g, c) in gpu.iter().zip(&reference) {
            assert!((g - c).abs() < 1e-5, "GPU {g} vs CPU {c}");
        }
    }

    #[test]
    fn scalar_threshold_gpu_matches_cpu() {
        let values = deterministic_scalars(1000);
        let gpu = scalar_threshold_gpu(&values, 5.0).expect("GPU 阈值掩码");
        for (g, &v) in gpu.iter().zip(&values) {
            assert_eq!(*g, if v >= 5.0 { 1.0 } else { 0.0 });
        }
    }

    #[test]
    fn scalar_difference_gpu_matches_cpu() {
        let a = deterministic_scalars(1000);
        let b = deterministic_scalars(1000);
        let gpu = scalar_difference_gpu(&a, &b).expect("GPU 两场差值");
        for ((g, &x), &y) in gpu.iter().zip(a.iter()).zip(b.iter()) {
            assert!((g - (x - y)).abs() < 1e-6);
        }
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
