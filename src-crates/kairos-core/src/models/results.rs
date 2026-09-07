//! 结果数据模型：OpenFOAM 求解输出的时间步目录与场数据（T13）。

use serde::Serialize;

/// 单个时间步元数据：目录名、物理时间、该目录下可读的场文件列表。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeStepMeta {
    /// OpenFOAM 时间目录名，如 "0"、"0.005"。
    pub dir_name: String,
    /// 物理时间（s）。
    pub time_s: f64,
    /// 该目录下可读的场文件名（如 T / p / U）。
    pub fields: Vec<String>,
}

/// 时间步目录清单（结果目录的元数据，先于场数据加载）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultCatalog {
    pub case_dir: String,
    pub times: Vec<TimeStepMeta>,
}

/// 已加载的标量场：矢量场（如 U）以模量形式给出（v1）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalarField {
    pub field: String,
    pub time_dir: String,
    pub time_s: f64,
    /// 与体积网格单元一一对应的值。
    pub values: Vec<f64>,
    /// true = 原始为矢量场，此处为模量。
    pub is_magnitude: bool,
    /// false = 声明数量与实际不符（求解中途取消的不完整结果）。
    pub complete: bool,
}
