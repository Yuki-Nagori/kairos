//! GPU 加速后处理算子：wgpu compute，接入派生命令生产路径（derive_field /
//! derive_difference）。CPU 参考实现住 core（services::operators / results::derive*），
//! 仅作正确性基准——后处理以硬件加速 GPU 为运行前提，无 CPU 运行时回退。
//! 算子清单（v1）：矢量模量、线性映射（含归一化）、阈值掩码、两场差值；
//! LOD / 切片按同一模式扩展。

use std::sync::OnceLock;

use kairos_core::error::KairosError;
use kairos_core::models::results::{DeriveRequest, ScalarField};
use serde::Serialize;
use wgpu::util::DeviceExt;

// 着色器源码外置在 src-tauri/shaders/（编译期 include_str! 内联），算子目录化铺路。
const VECTOR_MAGNITUDE_SHADER: &str = include_str!("../../shaders/vector_magnitude.wgsl");

const WORKGROUP_SIZE: usize = 64;

/// 单次 dispatch 的元素上限：Metal/Vulkan 限制每维 workgroup 数 ≤ 65535，
/// 超限结果场（> 4.19M 值，100MB 级结果即触发）必须按块切分多次 dispatch，
/// 否则 wgpu 校验直接 panic（GPU 消融计时实测发现）。
const MAX_ELEMENTS_PER_DISPATCH: usize = 65_535 * WORKGROUP_SIZE;

#[cfg(test)]
use kairos_core::services::{operators::vector_magnitude_cpu, results};

/// 进程级 compute 设备缓存。Device/Queue 本就为长生命周期设计，重建开销在
/// 百毫秒级，没必要每次算子调用重新枚举适配器；探测失败同样缓存——无 GPU
/// 环境下保持快速、明确的报错语义，不反复枚举。
static COMPUTE_DEVICE: OnceLock<Option<(wgpu::Device, wgpu::Queue)>> = OnceLock::new();

fn compute_device() -> Result<&'static (wgpu::Device, wgpu::Queue), KairosError> {
    COMPUTE_DEVICE
        .get_or_init(|| create_device().ok())
        .as_ref()
        .ok_or_else(|| {
            KairosError::internal(
                "后处理需要硬件加速 GPU：未检测到可用适配器，该能力不受支持（无 CPU 运行时回退）。",
            )
        })
}

fn create_device() -> Result<(wgpu::Device, wgpu::Queue), KairosError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| {
        KairosError::internal("后处理需要硬件加速 GPU：未检测到可用适配器，该能力不受支持。")
    })?;
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

/// 公共管线尾段：一次 compute pass + 输出拷贝 + MAP_READ 回读，返回 len 个 f32。
/// 所有算子共享同一编码方式（输出缓冲按 f32 LE 打包），差异只在管线与绑定。
fn dispatch_and_readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    output_buffer: &wgpu::Buffer,
    element_count: usize,
) -> Result<Vec<f32>, KairosError> {
    let byte_size = (element_count * 4) as u64;
    let read_back_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{label}_read_back")),
        size: byte_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let workgroups = element_count.div_ceil(WORKGROUP_SIZE) as u32;
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&format!("{label}_pass")),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(output_buffer, 0, &read_back_buffer, 0, byte_size);
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

/// GPU 算子：矢量模量。输入按 vec4 对齐；无 GPU 环境返回内部错误
/// （后处理无 CPU 运行时回退，调用方不降级）。
pub fn vector_magnitude_gpu(vectors: &[[f32; 3]]) -> Result<Vec<f32>, KairosError> {
    if vectors.is_empty() {
        return Ok(Vec::new());
    }
    let (device, queue) = compute_device()?;

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

    // vec4 对齐展开 + 超限按块切分多次 dispatch（管线只建一次），逐块回读拼接。
    let mut result = Vec::with_capacity(vectors.len());
    for chunk_start in (0..vectors.len()).step_by(MAX_ELEMENTS_PER_DISPATCH) {
        let chunk =
            &vectors[chunk_start..(chunk_start + MAX_ELEMENTS_PER_DISPATCH).min(vectors.len())];
        let mut flat: Vec<f32> = Vec::with_capacity(chunk.len() * 4);
        for vector in chunk {
            flat.extend_from_slice(&[vector[0], vector[1], vector[2], 0.0]);
        }
        let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector_magnitude_input"),
            contents: bytemuck::cast_slice(&flat),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector_magnitude_out"),
            contents: vec![0u8; chunk.len() * 4].as_slice(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
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
        result.extend(dispatch_and_readback(
            device,
            queue,
            "vector_magnitude",
            &pipeline,
            &bind_group,
            &output_buffer,
            chunk.len(),
        )?);
    }
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
    let (device, queue) = compute_device()?;

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

    // 超限结果场按块切分多次 dispatch（管线只建一次），逐块回读拼接。
    let mut result = Vec::with_capacity(len);
    for chunk_start in (0..len).step_by(MAX_ELEMENTS_PER_DISPATCH) {
        let chunk_len = MAX_ELEMENTS_PER_DISPATCH.min(len - chunk_start);
        result.extend(run_scalar_chunk(
            device,
            queue,
            label,
            &pipeline,
            &inputs[0][chunk_start..chunk_start + chunk_len],
            inputs
                .get(1)
                .map(|data| &data[chunk_start..chunk_start + chunk_len]),
            params,
            chunk_len,
        )?);
    }
    Ok(result)
}

/// 单块（≤ MAX_ELEMENTS_PER_DISPATCH 值）的缓冲构建 + dispatch + 回读。
#[allow(clippy::too_many_arguments)]
fn run_scalar_chunk(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    pipeline: &wgpu::ComputePipeline,
    input_a: &[f32],
    input_b: Option<&[f32]>,
    params: &[f32; 4],
    chunk_len: usize,
) -> Result<Vec<f32>, KairosError> {
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

    let input_a_buffer = make_buffer(
        &format!("{label}_in_a"),
        bytemuck::cast_slice(input_a),
        false,
    );
    let input_b_buffer = input_b
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
        contents: vec![0u8; chunk_len * 4].as_slice(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    // 两种绑定布局：带参数（线性/阈值：in@0, params@1, out@2）
    // 与双输入（差值：a@0, b@1, out@2，无参数缓冲）。
    let entries: Vec<wgpu::BindGroupEntry> = match &input_b_buffer {
        None => {
            let pb = params_buffer
                .as_ref()
                .ok_or_else(|| KairosError::internal("带参着色器缺少参数缓冲"))?;
            vec![
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_a_buffer.as_entire_binding(),
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
                resource: input_a_buffer.as_entire_binding(),
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

    dispatch_and_readback(
        device,
        queue,
        label,
        pipeline,
        &bind_group,
        &output_buffer,
        chunk_len,
    )
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

/// GPU 主路径：单场派生（归一化 / 阈值掩码 / 线性映射），命名与标志位与
/// core 的 CPU 参考实现（results::derive_scalar_field）逐字段一致。
/// f64 场转 f32 参与计算——可视化后处理精度；Difference 单场入口不受理。
pub fn derive_scalar_field_gpu(
    field: &ScalarField,
    request: &DeriveRequest,
) -> Result<ScalarField, KairosError> {
    if field.values.is_empty() {
        // 空场原样返回：上层保持名称与状态不变（与 CPU 参考一致）
        return Ok(field.clone());
    }
    // min/max 以 f64 统计（与 CPU 参考同口径），随后转 f32 进着色器。
    let min = field.values.iter().cloned().fold(f64::INFINITY, f64::min) as f32;
    let max = field
        .values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max) as f32;
    let values: Vec<f32> = field.values.iter().map(|&v| v as f32).collect();

    let (suffix, derived): (String, Vec<f32>) = match request {
        DeriveRequest::Normalize => {
            let range = max - min;
            if range <= 0.0 {
                ("归一化".into(), vec![0.0; values.len()])
            } else {
                // (v - min)/range ≡ v·(1/range) + (-min·(1/range))：复用线性映射管线
                let scale = 1.0 / range;
                (
                    "归一化".into(),
                    scalar_linear_gpu(&values, scale, -min * scale)?,
                )
            }
        }
        DeriveRequest::Threshold => (
            "阈值掩码".into(),
            scalar_threshold_gpu(&values, (min + max) / 2.0)?,
        ),
        DeriveRequest::Linear { scale, offset } => (
            format!("线性映射 ×{scale} {offset:+}"),
            scalar_linear_gpu(&values, *scale as f32, *offset as f32)?,
        ),
        // 差值需要主场与对比场两份数据，单场入口不受理（与 CPU 参考一致）。
        DeriveRequest::Difference => {
            return Err(KairosError::validation(
                "两场差值请使用 derive_difference 命令（需要主场与对比场）。",
            ));
        }
    };

    Ok(ScalarField {
        field: format!("{} · {suffix}", field.field),
        time_dir: field.time_dir.clone(),
        time_s: field.time_s,
        values: derived.into_iter().map(f64::from).collect(),
        is_magnitude: false,
        complete: field.complete,
    })
}

/// GPU 主路径：两场差值（主场 − 对比场）。长度校验、命名与标志位与
/// core 的 CPU 参考实现（results::derive_difference）一致。
pub fn derive_difference_gpu(
    primary: &ScalarField,
    compare: &ScalarField,
) -> Result<ScalarField, KairosError> {
    if primary.values.len() != compare.values.len() {
        return Err(KairosError::validation(format!(
            "两场长度不一致：{} 有 {} 个值，{} 有 {} 个值。",
            primary.field,
            primary.values.len(),
            compare.field,
            compare.values.len()
        )));
    }
    let a: Vec<f32> = primary.values.iter().map(|&v| v as f32).collect();
    let b: Vec<f32> = compare.values.iter().map(|&v| v as f32).collect();
    let derived = scalar_difference_gpu(&a, &b)?;
    Ok(ScalarField {
        field: format!("{} - {}", primary.field, compare.field),
        time_dir: primary.time_dir.clone(),
        time_s: primary.time_s,
        values: derived.into_iter().map(f64::from).collect(),
        is_magnitude: false,
        complete: primary.complete && compare.complete,
    })
}

/// CPU 参考实现转交（测试与一致性基准使用；生产路径不经过）。
#[cfg(test)]
fn derive_scalar_field_cpu(
    field: &ScalarField,
    request: &DeriveRequest,
) -> Result<ScalarField, kairos_core::error::KairosError> {
    results::derive_scalar_field(field, request)
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

    /// GPU 测试的 CI 门控：无适配器环境显式跳过（GPU 为运行前提、无 CPU
    /// 回退，但测试套件须可在无 GPU 的 CI runner 验证其余逻辑——D0
    /// 「GPU 必需但 CI 可验证」策略）。探测经进程级设备缓存复用。
    fn skip_without_gpu() -> bool {
        if compute_device().is_ok() {
            return false;
        }
        println!("跳过：无可用 GPU 适配器（无 GPU 环境按策略跳过，真机验收归 T48）");
        true
    }

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
        if skip_without_gpu() {
            return;
        }

        let values = deterministic_scalars(1000);
        let gpu = scalar_linear_gpu(&values, 0.5, -2.0).expect("GPU 线性映射");
        let reference: Vec<f32> = values.iter().map(|&v| v * 0.5 - 2.0).collect();
        for (g, c) in gpu.iter().zip(&reference) {
            assert!((g - c).abs() < 1e-5, "GPU {g} vs CPU {c}");
        }
    }

    #[test]
    fn scalar_threshold_gpu_matches_cpu() {
        if skip_without_gpu() {
            return;
        }

        let values = deterministic_scalars(1000);
        let gpu = scalar_threshold_gpu(&values, 5.0).expect("GPU 阈值掩码");
        for (g, &v) in gpu.iter().zip(&values) {
            assert_eq!(*g, if v >= 5.0 { 1.0 } else { 0.0 });
        }
    }

    #[test]
    fn scalar_difference_gpu_matches_cpu() {
        if skip_without_gpu() {
            return;
        }

        let a = deterministic_scalars(1000);
        let b = deterministic_scalars(1000);
        let gpu = scalar_difference_gpu(&a, &b).expect("GPU 两场差值");
        for ((g, &x), &y) in gpu.iter().zip(a.iter()).zip(b.iter()) {
            assert!((g - (x - y)).abs() < 1e-6);
        }
    }

    #[test]
    fn gpu_magnitudes_match_cpu_reference() {
        if skip_without_gpu() {
            return;
        }

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

    // ─── GPU 生产路径（derive_*_gpu）与 core CPU 参考的一致性 ───

    fn sample_field(name: &str, count: usize) -> ScalarField {
        let values: Vec<f64> = deterministic_scalars(count)
            .into_iter()
            .map(f64::from)
            .collect();
        ScalarField {
            field: name.into(),
            time_dir: "100".into(),
            time_s: 1.0,
            values,
            is_magnitude: false,
            complete: true,
        }
    }

    fn assert_fields_close(gpu: &ScalarField, cpu: &ScalarField) {
        assert_eq!(gpu.field, cpu.field, "派生命名必须与 CPU 参考一致");
        assert_eq!(gpu.time_dir, cpu.time_dir);
        assert_eq!(gpu.time_s, cpu.time_s);
        assert_eq!(gpu.is_magnitude, cpu.is_magnitude);
        assert_eq!(gpu.complete, cpu.complete);
        assert_eq!(gpu.values.len(), cpu.values.len());
        for (g, c) in gpu.values.iter().zip(&cpu.values) {
            assert!((g - c).abs() < 1e-5, "GPU {g} vs CPU {c}");
        }
    }

    #[test]
    fn derive_normalize_gpu_matches_cpu_reference() {
        if skip_without_gpu() {
            return;
        }

        let field = sample_field("T", 256);
        let gpu = derive_scalar_field_gpu(&field, &DeriveRequest::Normalize).expect("GPU 归一化");
        let cpu = derive_scalar_field_cpu(&field, &DeriveRequest::Normalize).unwrap();
        assert_fields_close(&gpu, &cpu);
    }

    #[test]
    fn derive_threshold_gpu_matches_cpu_reference() {
        if skip_without_gpu() {
            return;
        }

        let field = sample_field("T", 256);
        let gpu = derive_scalar_field_gpu(&field, &DeriveRequest::Threshold).expect("GPU 阈值");
        let cpu = derive_scalar_field_cpu(&field, &DeriveRequest::Threshold).unwrap();
        assert_fields_close(&gpu, &cpu);
    }

    #[test]
    fn derive_linear_gpu_matches_cpu_reference() {
        if skip_without_gpu() {
            return;
        }

        let field = sample_field("p", 256);
        let request = DeriveRequest::Linear {
            scale: 2.0,
            offset: -1.0,
        };
        let gpu = derive_scalar_field_gpu(&field, &request).expect("GPU 线性映射");
        let cpu = derive_scalar_field_cpu(&field, &request).unwrap();
        assert_fields_close(&gpu, &cpu);
    }

    #[test]
    fn derive_normalize_flat_field_maps_to_zeros_on_gpu() {
        if skip_without_gpu() {
            return;
        }

        let field = ScalarField {
            field: "T".into(),
            time_dir: "100".into(),
            time_s: 1.0,
            values: vec![5.0; 64],
            is_magnitude: false,
            complete: true,
        };
        let gpu = derive_scalar_field_gpu(&field, &DeriveRequest::Normalize).expect("GPU 归一化");
        assert!(gpu.values.iter().all(|&v| v == 0.0));
        assert!(gpu.field.contains("归一化"));
    }

    #[test]
    fn derive_scalar_field_gpu_rejects_difference_request() {
        let field = sample_field("T", 8);
        let error = derive_scalar_field_gpu(&field, &DeriveRequest::Difference).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("两场差值请使用 derive_difference 命令")
        );
    }

    #[test]
    fn derive_difference_gpu_matches_cpu_reference() {
        if skip_without_gpu() {
            return;
        }

        let primary = sample_field("T", 256);
        let compare = sample_field("T0", 256);
        let gpu = derive_difference_gpu(&primary, &compare).expect("GPU 两场差值");
        let cpu = results::derive_difference(&primary, &compare).unwrap();
        assert_fields_close(&gpu, &cpu);
    }

    #[test]
    fn derive_difference_gpu_rejects_length_mismatch() {
        let error =
            derive_difference_gpu(&sample_field("T", 8), &sample_field("T0", 16)).unwrap_err();
        assert!(error.to_string().contains("两场长度不一致"));
    }

    #[test]
    fn derive_gpu_handles_over_dispatch_limit_lengths() {
        if skip_without_gpu() {
            return;
        }

        // 回归：> 4.19M 值时单次 dispatch 超 Metal/Vulkan 每维 65535 workgroup
        // 硬限会 panic（消融计时实测）——按块切分后必须与 CPU 参考一致。
        let n = MAX_ELEMENTS_PER_DISPATCH + 7;
        let mut state: u32 = 0x7654_3210;
        let values: Vec<f64> = (0..n)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                ((state >> 8) as f64) / 16_777_216.0 * 50.0
            })
            .collect();
        let field = ScalarField {
            field: "T".into(),
            time_dir: "1".into(),
            time_s: 1.0,
            values,
            is_magnitude: false,
            complete: true,
        };
        let gpu = derive_scalar_field_gpu(
            &field,
            &DeriveRequest::Linear {
                scale: 2.0,
                offset: -1.0,
            },
        )
        .expect("GPU 超限长度");
        let cpu = results::derive_scalar_field(
            &field,
            &DeriveRequest::Linear {
                scale: 2.0,
                offset: -1.0,
            },
        )
        .unwrap();
        assert_eq!(gpu.values.len(), cpu.values.len());
        for (g, c) in gpu.values.iter().zip(&cpu.values) {
            assert!((g - c).abs() < 1e-5, "GPU {g} vs CPU {c}");
        }
    }

    /// GPU/CPU 消融计时（perf-budget「GPU 辅助算子 ≥5×」预算的测量入口）：
    /// `cargo test -p kairos --lib --release gpu_derive_ablation -- --ignored --nocapture`
    /// 冷启动含 device 创建（缓存消融的「无缓存」对照），warm 为 OnceLock 缓存后的
    /// 稳态；CPU 为 core 参考实现（唯一事实源）。确定性数据，数字可复现可横向比较。
    /// 规模扫描（1e5 / 1e6 / 1e7 / 3e7 值）用于定位 GPU 相对 CPU 的数据量交叉点。
    #[test]
    #[ignore]
    fn gpu_derive_ablation_timing() {
        let request = DeriveRequest::Linear {
            scale: 2.0,
            offset: -1.0,
        };
        // 冷启动：进程内第一次 GPU 调用含 wgpu device 枚举与创建（OnceLock 缓存的
        // 「无缓存」对照——每次调用重建 device 即每块都付出这一成本）。
        let mut cold: Option<std::time::Duration> = None;
        let mut state: u32 = 0xABCD_1234;
        let mut rng = move || {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            ((state >> 8) as f64) / 16_777_216.0 * 100.0
        };

        println!();
        println!("=== GPU/CPU 派生消融（Linear 2.0/-1.0，确定性数据）===");
        println!(
            "{:>10} {:>14} {:>12} {:>12} {:>10}",
            "规模", "GPU cold", "GPU warm", "CPU", "CPU/GPU"
        );

        for &n in &[100_000usize, 1_000_000, 10_000_000, 30_000_000] {
            let values: Vec<f64> = (0..n).map(|_| rng()).collect();
            let field = ScalarField {
                field: "T".into(),
                time_dir: "bench".into(),
                time_s: 0.0,
                values,
                is_magnitude: false,
                complete: true,
            };

            // 冷启动只发生在首个规模段。
            if cold.is_none() {
                let t0 = std::time::Instant::now();
                let r = derive_scalar_field_gpu(&field, &request).expect("GPU cold");
                assert_eq!(r.values.len(), n);
                cold = Some(t0.elapsed());
            }

            // 稳态：缓存后的 GPU 派生（5 次取均值）。
            let t0 = std::time::Instant::now();
            for _ in 0..5 {
                let _ = derive_scalar_field_gpu(&field, &request).expect("GPU warm");
            }
            let warm = t0.elapsed() / 5;

            // CPU 参考实现（消融 GPU 后的唯一可用路径，作为对照基准）。
            let t0 = std::time::Instant::now();
            let cpu_result = results::derive_scalar_field(&field, &request).unwrap();
            let cpu_time = t0.elapsed();
            assert_eq!(cpu_result.values.len(), n);

            let speedup = cpu_time.as_secs_f64() / warm.as_secs_f64();
            match cold {
                Some(c) => println!(
                    "{n:>10} {:>13?} {:>12?} {:>12?} {:>9.1}×",
                    c, warm, cpu_time, speedup
                ),
                None => println!(
                    "{n:>10} {:>14} {:>12?} {:>12?} {:>9.1}×",
                    "—", warm, cpu_time, speedup
                ),
            }
        }
    }

    #[test]
    fn derive_gpu_handles_empty_field_like_cpu() {
        let field = sample_field("T", 0);
        let gpu = derive_scalar_field_gpu(&field, &DeriveRequest::Normalize).unwrap();
        let cpu = derive_scalar_field_cpu(&field, &DeriveRequest::Normalize).unwrap();
        assert!(gpu.values.is_empty());
        assert_eq!(gpu.field, cpu.field);

        let difference = derive_difference_gpu(&field, &field).unwrap();
        assert!(difference.values.is_empty());
        assert_eq!(difference.field, "T - T");
    }
}
