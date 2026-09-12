//! GPU 命令：wgpu 适配器探测与厂商识别（NVIDIA / AMD / Intel / Apple）。
//! 统一走 wgpu（Vulkan / DX12 / Metal），禁止引入 CUDA 等单厂商 SDK。

use kairos_core::error::Result;
use serde::Serialize;

/// GPU 适配器信息（诊断页展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuAdapterInfo {
    pub name: String,
    /// 后端：vulkan / dx12 / metal / browser_webgpu。
    pub backend: String,
    /// 厂商：nvidia / amd / intel / apple / unknown。
    pub vendor: String,
    pub device_type: String,
}

#[tauri::command]
pub async fn probe_gpu() -> Result<Vec<GpuAdapterInfo>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let mut adapters = Vec::new();
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        adapters.push(GpuAdapterInfo {
            name: info.name,
            backend: format!("{:?}", info.backend).to_lowercase(),
            vendor: vendor_name(info.vendor),
            device_type: format!("{:?}", info.device_type).to_lowercase(),
        });
    }
    Ok(adapters)
}

/// PCI 厂商 ID → 厂商名（诊断展示用）。
fn vendor_name(vendor_id: u32) -> String {
    match vendor_id {
        0x10DE => "nvidia".into(),
        0x1002 => "amd".into(),
        0x8086 => "intel".into(),
        0x106B => "apple".into(),
        other => format!("unknown({other:#x})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_ids_map_to_names() {
        assert_eq!(vendor_name(0x10DE), "nvidia");
        assert_eq!(vendor_name(0x1002), "amd");
        assert_eq!(vendor_name(0x8086), "intel");
        assert_eq!(vendor_name(0x106B), "apple");
        assert!(vendor_name(0x1234).starts_with("unknown"));
    }

    #[test]
    fn probe_returns_adapters_on_machines_with_gpu() {
        // 在有 GPU 的机器上（开发机与 macOS CI），至少能枚举一个适配器；
        // 无适配器环境按 D0「GPU 必需但 CI 可验证」策略显式跳过。
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        if instance
            .enumerate_adapters(wgpu::Backends::all())
            .is_empty()
        {
            println!("跳过：无可用 GPU 适配器（无 GPU 环境按策略跳过）");
            return;
        }
        let adapters = pollster::block_on(probe_gpu()).expect("GPU 探测不应失败");
        assert!(!adapters.is_empty(), "应有至少一个可用适配器");
    }
}
