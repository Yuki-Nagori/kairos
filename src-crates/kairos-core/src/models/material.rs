//! 材料模型：热塑性塑料牌号，参数组与注塑模拟的标准物理输入对齐
//! （Cross-WLF 黏度、Tait PVT、温度表热物性）。

use serde::{Deserialize, Serialize};

/// Cross-WLF 七参数黏度模型（剪切变稀 + WLF 温度移位）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossWlf {
    /// 非牛顿指数 (0, 1]。
    pub n: f64,
    /// 参考剪切应力 τ*（Pa）。
    pub tau_star: f64,
    /// 零剪切黏度前置系数 D1（Pa·s）。
    pub d1: f64,
    /// 极限温度 D2（K）。
    pub d2: f64,
    /// 压力依赖项 D3（K/Pa）。
    pub d3: f64,
    /// WLF 参数 A1。
    pub a1: f64,
    /// WLF 参数 A2（K）。
    pub a2: f64,
}

/// Tait 型 PVT 方程参数（比容，m³/kg；压力 Pa；温度 K）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tait {
    pub b1m: f64,
    pub b1s: f64,
    pub b2m: f64,
    pub b2s: f64,
    pub b3: f64,
    pub b4m: f64,
    pub b4s: f64,
    /// 温度转变参数 b5（K）。
    pub b5: f64,
}

/// 温度相关的标量性质表（温度 K，值；温度须严格递增）。
pub type PropertyTable = Vec<(f64, f64)>;

/// 材料力学参数（为后续结构/翘曲分析预留，允许缺省）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mechanics {
    /// 弹性模量（Pa）。
    pub elastic_modulus: f64,
    /// 泊松比 (0, 0.5)。
    pub poisson_ratio: f64,
}

/// 纤维 / 填料参数组（玻纤、滑石粉等增强 / 填充体系；无填料牌号缺省）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillerGroup {
    /// 填料类型，如「玻纤」「碳纤维」「滑石粉」。
    pub kind: String,
    /// 质量分数 (0, 1]。
    pub weight_fraction: f64,
    /// 平均长径比（非纤维填料填 1）。
    pub aspect_ratio: f64,
    /// 数据来源或工艺提示。
    pub note: String,
}

/// 一个材料牌号的完整物理描述。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    /// 牌号名，如「PP-HM-001」。
    pub name: String,
    pub manufacturer: String,
    /// 材料族缩写，如 PP / ABS / PC。
    pub family: String,
    pub rheology: CrossWlf,
    pub pvt: Tait,
    /// 恒压比热表 (T, Cp J/(kg·K))。
    pub specific_heat: PropertyTable,
    /// 导热系数表 (T, λ W/(m·K))。
    pub conductivity: PropertyTable,
    /// 力学参数（远期翘曲/结构分析使用）。
    pub mechanics: Option<Mechanics>,
    /// 纤维 / 填料参数组（无填料牌号为 null）。
    pub filler: Option<FillerGroup>,
    /// 数据来源与免责声明（内置示例材料必须标注）。
    pub data_note: String,
}
