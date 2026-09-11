//! 材料领域服务：参数校验、内置参考牌号库、自定义材料库的读写规则。

use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::material::{Material, PropertyTable};

/// 内置参考牌号（数据为公开文献典型值，非实测；交付给用户前必须保留免责声明）。
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
    if let Some(filler) = &material.filler {
        if filler.kind.trim().is_empty() {
            return Err(KairosError::validation("填料类型不能为空。"));
        }
        if !(filler.weight_fraction > 0.0 && filler.weight_fraction <= 1.0) {
            return Err(KairosError::validation("填料质量分数应在 (0, 1] 区间。"));
        }
        if filler.aspect_ratio <= 0.0 {
            return Err(KairosError::validation("填料长径比必须为正数。"));
        }
    }
    Ok(())
}

/// 解析内置参考牌号库；资产内嵌于二进制，损坏属构建期错误，直接快速失败。
pub fn builtin_materials() -> Vec<Material> {
    serde_json::from_str(BUILTIN_MATERIALS_JSON).expect("内置材料资产损坏")
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

/// CSV 批量导入的约定表头（列序固定；比热 / 导热为「温度:值」分号表，
/// filler 列为「类型:质量分数:长径比:备注」且可留空；字段内不得包含逗号）。
const CSV_HEADER: [&str; 21] = [
    "name",
    "manufacturer",
    "family",
    "n",
    "tauStar",
    "d1",
    "d2",
    "d3",
    "a1",
    "a2",
    "b1m",
    "b1s",
    "b2m",
    "b2s",
    "b3",
    "b4m",
    "b4s",
    "b5",
    "specificHeat",
    "conductivity",
    "filler",
];

/// 解析 CSV 批量导入内容（表头 + 数据行），逐行校验并生成材料 id。
pub fn parse_custom_csv(content: &str) -> Result<Vec<Material>> {
    use crate::services::project::new_id;

    let mut lines = content.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| KairosError::validation("CSV 缺少表头行。"))?;
    let columns: Vec<&str> = header.split(',').map(str::trim).collect();
    if columns != CSV_HEADER {
        return Err(KairosError::validation(
            "CSV 表头与约定列序不一致，请使用导入面板提供的模板列名。",
        ));
    }

    let mut materials = Vec::new();
    for (offset, line) in lines.enumerate() {
        let row_no = offset + 2;
        let row: Vec<&str> = line.split(',').map(str::trim).collect();
        if row.len() != CSV_HEADER.len() {
            return Err(KairosError::validation(format!(
                "CSV 第 {row_no} 行的列数与表头不一致。"
            )));
        }
        let number = |column: usize, label: &str| -> Result<f64> {
            row[column].parse::<f64>().map_err(|e| {
                KairosError::validation(format!("第 {row_no} 行 {label} 不是数字：{e}"))
            })
        };
        let table = |column: usize, label: &str| -> Result<PropertyTable> {
            let mut pairs = Vec::new();
            for pair in row[column].split(';').filter(|p| !p.trim().is_empty()) {
                let (temperature, value) = pair.split_once(':').ok_or_else(|| {
                    KairosError::validation(format!(
                        "第 {row_no} 行 {label} 的「{pair}」缺少冒号分隔。"
                    ))
                })?;
                let point = (
                    temperature.trim().parse::<f64>().map_err(|e| {
                        KairosError::validation(format!("第 {row_no} 行 {label} 温度不是数字：{e}"))
                    })?,
                    value.trim().parse::<f64>().map_err(|e| {
                        KairosError::validation(format!("第 {row_no} 行 {label} 值不是数字：{e}"))
                    })?,
                );
                pairs.push(point);
            }
            Ok(pairs)
        };
        let filler = if row[20].is_empty() {
            None
        } else {
            let parts: Vec<&str> = row[20].splitn(4, ':').collect();
            if parts.len() != 4 {
                return Err(KairosError::validation(format!(
                    "第 {row_no} 行 filler 应为「类型:质量分数:长径比:备注」。"
                )));
            }
            let weight_fraction = parts[1].trim().parse::<f64>().map_err(|e| {
                KairosError::validation(format!("第 {row_no} 行 填料质量分数不是数字：{e}"))
            })?;
            let aspect_ratio = parts[2].trim().parse::<f64>().map_err(|e| {
                KairosError::validation(format!("第 {row_no} 行 填料长径比不是数字：{e}"))
            })?;
            Some(crate::models::material::FillerGroup {
                kind: parts[0].trim().to_string(),
                weight_fraction,
                aspect_ratio,
                note: parts[3].trim().to_string(),
            })
        };

        let material = Material {
            id: new_id("mat"),
            name: row[0].to_string(),
            manufacturer: row[1].to_string(),
            family: row[2].to_string(),
            rheology: crate::models::material::CrossWlf {
                n: number(3, "n")?,
                tau_star: number(4, "tauStar")?,
                d1: number(5, "d1")?,
                d2: number(6, "d2")?,
                d3: number(7, "d3")?,
                a1: number(8, "a1")?,
                a2: number(9, "a2")?,
            },
            pvt: crate::models::material::Tait {
                b1m: number(10, "b1m")?,
                b1s: number(11, "b1s")?,
                b2m: number(12, "b2m")?,
                b2s: number(13, "b2s")?,
                b3: number(14, "b3")?,
                b4m: number(15, "b4m")?,
                b4s: number(16, "b4s")?,
                b5: number(17, "b5")?,
            },
            specific_heat: table(18, "specificHeat")?,
            conductivity: table(19, "conductivity")?,
            mechanics: None,
            filler,
            data_note: "CSV 批量导入".to_string(),
        };
        validate(&material)?;
        materials.push(material);
    }
    if materials.is_empty() {
        return Err(KairosError::validation("CSV 中没有数据行。"));
    }
    Ok(materials)
}

/// 序列化自定义材料集（写入用户材料库文件）。
pub fn serialize_custom(materials: &[Material]) -> Result<String> {
    for material in materials {
        validate(material)?;
    }
    // 已通过校验的材料必可序列化；失败属程序缺陷，快速失败。
    Ok(serde_json::to_string_pretty(materials).expect("材料库序列化失败"))
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
    use crate::models::material::{CrossWlf, FillerGroup, Mechanics, Tait};

    fn valid_material() -> Material {
        serde_json::from_str(&serde_json::to_string(&builtin_materials()[0]).unwrap()).unwrap()
    }

    #[test]
    fn builtin_assets_are_valid() {
        let materials = builtin_materials();
        assert!(materials.len() >= 2, "内置参考牌号至少 2 个");
        for material in &materials {
            assert!(validate(material).is_ok(), "{} 校验失败", material.name);
            assert!(!material.data_note.is_empty(), "参考牌号必须注明数据来源");
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
        let materials = builtin_materials();
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

    #[test]
    fn csv_import_parses_rows_and_filler() {
        let content = format!(
            "{header}\n\n演示牌号,示例厂,PP,0.35,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900;400:2200,300:0.2;400:0.18,\n玻纤牌号,示例厂,PA66-GF30,0.26,34000,3e12,323,0,31,51.6,1.33e-3,1.29e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,560,300:1600,300:0.35,玻纤:0.3:20:短切玻纤",
            header = CSV_HEADER.join(",")
        );
        let materials = parse_custom_csv(&content).unwrap();
        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].name, "演示牌号");
        assert!(materials[0].filler.is_none());
        let filler = materials[1].filler.as_ref().unwrap();
        assert_eq!(filler.kind, "玻纤");
        assert!((filler.weight_fraction - 0.3).abs() < 1e-9);
        assert!(materials.iter().all(|m| validate(m).is_ok()));
    }

    #[test]
    fn csv_import_rejects_bad_header_row_count_and_number() {
        let bad_header = "a,b,c\n1,2,3";
        let error = parse_custom_csv(bad_header).unwrap_err();
        assert!(error.to_string().contains("表头"));

        let good_header = CSV_HEADER.join(",");
        let bad_count = format!("{good_header}\n只有三列");
        let error = parse_custom_csv(&bad_count).unwrap_err();
        assert!(error.to_string().contains("列数"));

        let bad_number = format!(
            "{good_header}\n牌号,厂,PP,x,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900,300:0.2,"
        );
        let error = parse_custom_csv(&bad_number).unwrap_err();
        assert!(error.to_string().contains("不是数字"));

        let empty_body = format!("{good_header}\n");
        let error = parse_custom_csv(&empty_body).unwrap_err();
        assert!(error.to_string().contains("没有数据行"));
    }

    #[test]
    fn csv_import_rejects_missing_header() {
        let error = parse_custom_csv("").unwrap_err();
        assert!(error.to_string().contains("缺少表头"));
    }

    #[test]
    fn csv_import_rejects_malformed_table_cell() {
        let good_header = CSV_HEADER.join(",");
        let row = |specific_heat: &str, conductivity: &str| {
            format!(
                "{good_header}\n牌号,厂,PP,0.35,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,{specific_heat},{conductivity},"
            )
        };
        let error = parse_custom_csv(&row("300:1900", "没有冒号")).unwrap_err();
        assert!(error.to_string().contains("缺少冒号分隔"));
        let error = parse_custom_csv(&row("温度:1900", "300:0.2")).unwrap_err();
        assert!(error.to_string().contains("温度不是数字"));
        let error = parse_custom_csv(&row("300:1900", "300:热导")).unwrap_err();
        assert!(error.to_string().contains("值不是数字"));
    }

    #[test]
    fn csv_import_rejects_malformed_filler_column() {
        let good_header = CSV_HEADER.join(",");
        let row = |filler: &str| {
            format!(
                "{good_header}\n牌号,厂,PP,0.35,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900,300:0.2,{filler}"
            )
        };
        let error = parse_custom_csv(&row("玻纤:0.3")).unwrap_err();
        assert!(error.to_string().contains("filler 应为"));
        let error = parse_custom_csv(&row("玻纤:abc:20:短切")).unwrap_err();
        assert!(error.to_string().contains("填料质量分数不是数字"));
        let error = parse_custom_csv(&row("玻纤:0.3:abc:短切")).unwrap_err();
        assert!(error.to_string().contains("填料长径比不是数字"));
    }

    #[test]
    fn validate_rejects_bad_filler_group() {
        let mut material = valid_material();
        let filler = |kind: &str, weight_fraction: f64, aspect_ratio: f64| FillerGroup {
            kind: kind.into(),
            weight_fraction,
            aspect_ratio,
            note: String::new(),
        };
        material.filler = Some(filler("   ", 0.3, 20.0));
        let error = validate(&material).unwrap_err();
        assert!(error.to_string().contains("填料类型不能为空"));
        material.filler = Some(filler("玻纤", 1.5, 20.0));
        let error = validate(&material).unwrap_err();
        assert!(error.to_string().contains("填料质量分数"));
        material.filler = Some(filler("玻纤", 0.3, 0.0));
        let error = validate(&material).unwrap_err();
        assert!(error.to_string().contains("填料长径比"));
    }
}
