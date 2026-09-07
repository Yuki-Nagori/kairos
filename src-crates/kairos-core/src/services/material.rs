//! 材料领域服务：参数校验、内置示例材料、自定义材料库的读写规则。

use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::material::{Material, PropertyTable};

/// 内置示例材料（数据为量级合理的示例值，非实测；交付给用户前必须保留免责声明）。
pub const BUILTIN_MATERIALS_JSON: &str = include_str!("../../assets/builtin-materials.json");

/// 校验温度表：非空、温度严格递增、数值有限且为正。
fn validate_table(table: &PropertyTable, label: &str) -> std::result::Result<(), String> {
    if table.is_empty() {
        return Err(format!("{label}不能为空。"));
    }
    let mut previous: Option<f64> = None;
    for (temperature, value) in table {
        if !temperature.is_finite() || !value.is_finite() || *value <= 0.0 {
            return Err(format!("{label}含非法数值（须为有限正数）。"));
        }
        if let Some(previous) = previous
            && *temperature <= previous
        {
            return Err(format!("{label}温度必须严格递增。"));
        }
        previous = Some(*temperature);
    }
    Ok(())
}

/// 校验一个材料的全部物理参数，返回首条错误的可读原因。
pub fn validate(material: &Material) -> Result<()> {
    let name = material.name.trim();
    if name.is_empty() {
        return Err(KairosError::validation("材料牌号不能为空。"));
    }
    if material.family.trim().is_empty() {
        return Err(KairosError::validation("材料族不能为空。"));
    }
    let r = &material.rheology;
    if !(0.0 < r.n && r.n <= 1.0) {
        return Err(KairosError::validation(
            "Cross-WLF 非牛顿指数 n 必须在 (0, 1]。",
        ));
    }
    if r.tau_star <= 0.0 || r.d1 <= 0.0 || r.a1 <= 0.0 {
        return Err(KairosError::validation(
            "Cross-WLF 的 τ*、D1、A1 必须为正数。",
        ));
    }
    if r.d2 <= 0.0 {
        return Err(KairosError::validation("Cross-WLF 的 D2 必须为正数（K）。"));
    }
    let t = &material.pvt;
    if !(t.b1m > 0.0
        && t.b1s > 0.0
        && t.b2m > 0.0
        && t.b2s > 0.0
        && t.b3 > 0.0
        && t.b4m > 0.0
        && t.b4s > 0.0)
    {
        return Err(KairosError::validation("Tait PVT 参数必须为正数。"));
    }
    if t.b1s >= t.b1m {
        return Err(KairosError::validation("固态比容 b1s 应小于熔态比容 b1m。"));
    }
    if t.b5 <= 0.0 {
        return Err(KairosError::validation(
            "Tait 转变温度 b5 必须为正数（K）。",
        ));
    }
    validate_table(&material.specific_heat, "比热表").map_err(KairosError::validation)?;
    validate_table(&material.conductivity, "导热系数表").map_err(KairosError::validation)?;
    Ok(())
}

/// 解析内置示例材料；内置资产损坏属于构建期错误，转 internal。
pub fn builtin_materials() -> Result<Vec<Material>> {
    serde_json::from_str(BUILTIN_MATERIALS_JSON)
        .map_err(|e| KairosError::internal(format!("内置材料资产损坏：{e}")))
}

/// 从 JSON 内容解析自定义材料集（单个对象或数组均可）。
pub fn parse_custom(content: &str) -> Result<Vec<Material>> {
    let parse_array = |value: Vec<Material>| -> Result<Vec<Material>> {
        for material in &value {
            validate(material)?;
        }
        Ok(value)
    };
    if let Ok(materials) = serde_json::from_str::<Material>(content) {
        return parse_array(vec![materials]);
    }
    match serde_json::from_str::<Vec<Material>>(content) {
        Ok(materials) => parse_array(materials),
        Err(e) => Err(KairosError::validation(format!(
            "材料 JSON 无法解析（且不是单个材料对象）：{e}"
        ))),
    }
}

/// 序列化自定义材料集（写入用户材料库文件）。
pub fn serialize_custom(materials: &[Material]) -> Result<String> {
    for material in materials {
        validate(material)?;
    }
    serde_json::to_string_pretty(materials)
        .map_err(|e| KairosError::internal(format!("材料库序列化失败：{e}")))
}

/// 把导入的材料合并进既有材料库：同 id 覆盖，同名同厂商提示冲突由调用方决定（此处按覆盖）。
pub fn merge_custom(existing: Vec<Material>, incoming: Vec<Material>) -> Vec<Material> {
    let mut merged = existing;
    for material in incoming {
        match merged.iter_mut().find(|m| m.id == material.id) {
            Some(slot) => *slot = material,
            None => merged.push(material),
        }
    }
    merged
}

/// 读取自定义材料库文件；文件缺失按空库处理。
pub fn read_custom_file(path: &Path) -> Vec<Material> {
    fs::read_to_string(path)
        .ok()
        .map(|content| parse_custom(&content).unwrap_or_default())
        .unwrap_or_default()
}

/// 原子写入自定义材料库文件。
pub fn write_custom_file(path: &Path, materials: &[Material]) -> Result<()> {
    let content = serialize_custom(materials)?;
    crate::services::project::write_atomic(path, &content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::material::{CrossWlf, Mechanics, Tait};

    fn valid_material() -> Material {
        serde_json::from_str(&serde_json::to_string(&builtin_materials().unwrap()[0]).unwrap())
            .unwrap()
    }

    #[test]
    fn builtin_assets_are_valid() {
        let materials = builtin_materials().unwrap();
        assert!(materials.len() >= 2, "内置示例材料至少 2 个");
        for material in &materials {
            validate(material).unwrap_or_else(|e| panic!("{} 校验失败：{e}", material.name));
            assert!(!material.data_note.is_empty(), "示例材料必须注明数据来源");
        }
    }

    #[test]
    fn validate_rejects_bad_rheology() {
        let mut material = valid_material();
        material.rheology.n = 1.5;
        let error = validate(&material).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }

    #[test]
    fn validate_rejects_unordered_table() {
        let mut material = valid_material();
        material.specific_heat = vec![(500.0, 2000.0), (400.0, 2100.0)];
        assert!(validate(&material).is_err());
    }

    #[test]
    fn parse_custom_accepts_single_and_array() {
        let material = valid_material();
        let single = serde_json::to_string(&material).unwrap();
        assert_eq!(parse_custom(&single).unwrap().len(), 1);
        let array = serde_json::to_string(&vec![material]).unwrap();
        assert_eq!(parse_custom(&array).unwrap().len(), 1);
    }

    #[test]
    fn merge_custom_overrides_by_id() {
        let mut existing = vec![valid_material()];
        let mut incoming = valid_material();
        incoming.name = "覆盖版".into();
        incoming.mechanics = Some(Mechanics {
            elastic_modulus: 1.5e9,
            poisson_ratio: 0.35,
        });
        let mut cross: CrossWlf = incoming.rheology.clone();
        cross.n = 0.3;
        incoming.rheology = cross;
        let _: Option<Tait> = None;

        let merged = merge_custom(std::mem::take(&mut existing), vec![incoming]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "覆盖版");
        assert!((merged[0].rheology.n - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_file_roundtrip_and_tolerant_read() {
        let path = std::env::temp_dir().join(format!("kairos-t04-{}.json", 1));
        let materials = builtin_materials().unwrap();
        write_custom_file(&path, &materials).unwrap();
        assert_eq!(read_custom_file(&path).len(), materials.len());
        fs::write(&path, "垃圾内容").unwrap();
        assert!(read_custom_file(&path).is_empty());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn property_table_validation_basics() {
        let mut material = valid_material();
        material.specific_heat = Vec::new();
        assert!(validate(&material).is_err());
    }
}
