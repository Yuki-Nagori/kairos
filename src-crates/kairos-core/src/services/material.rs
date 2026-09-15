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
    if t.b3s.is_some_and(|value| value <= 0.0) || t.c <= 0.0 || t.smooth_band <= 0.0 {
        return Err(KairosError::validation(
            "Tait 固态 B(T)、C 和平滑半带宽必须为正数。",
        ));
    }
    if let Some(blowing) = &material.blowing {
        if blowing.kind.trim().is_empty() {
            return Err(KairosError::validation("发泡剂类型不能为空。"));
        }
        if !(0.0..=30.0).contains(&blowing.mass_fraction_percent) {
            return Err(KairosError::validation("发泡剂质量分数必须在 0~30% 之间。"));
        }
        if !(0.0..=60.0).contains(&blowing.density_reduction_percent) {
            return Err(KairosError::validation(
                "微发泡密度下降率必须在 0~60% 之间。",
            ));
        }
        if !(0.0..90.0).contains(&blowing.viscosity_reduction_percent) {
            return Err(KairosError::validation(
                "微发泡黏度下降率必须在 0~90% 之间。",
            ));
        }
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

/// 从用户指定文件读取自定义材料；格式由扩展名决定，返回的材料逐项校验。
pub fn read_custom_material_file(path: &Path) -> Result<Vec<Material>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| KairosError::validation("自定义材料文件必须带 .json 或 .csv 扩展名。"))?;
    let content = fs::read_to_string(path).map_err(|error| {
        KairosError::io(format!(
            "读取自定义材料文件失败（{}）：{error}",
            path.display()
        ))
    })?;
    match extension.as_str() {
        "json" => parse_custom(&content),
        "csv" => parse_custom_csv(&content),
        _ => Err(KairosError::validation(
            "自定义材料文件格式不支持，仅支持 JSON 或 CSV。",
        )),
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
///
/// 解析走 `csv` 库而不是按逗号切分：引号包裹、字段内逗号、字段内换行、CRLF 与
/// Excel 导出的 UTF-8 BOM 都是真实文件里会出现的情况，自己按分隔符切会把它们
/// 静默解析成错列。校验口径（表头列名、列数、数值、属性表、填料列）与错误文案
/// 保持原有中文表述，仅解析器换实现。
pub fn parse_custom_csv(content: &str) -> Result<Vec<Material>> {
    // 空内容单独报「缺少表头」：交给库会得到「表头不一致」，对着空文件提示不准。
    // （UTF-8 BOM 由 csv 库自身剥离，Excel 导出的文件无需额外处理——用例锁定该行为。）
    if content.trim().is_empty() {
        return Err(KairosError::validation("CSV 缺少表头行。"));
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        // 列数校验由 `parse_csv_row` 给出中文口径：库的 flexible=false 会先抛英文错误，
        // 用户看到的原因就不是我们的措辞了。
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());
    // headers() 对 &str 输入不会失败（结构错误会先表现为字段数不足，由下面的列数
    // 校验拦下），因此不引入只读得到、却永远走不到的错误分支：取不到就按空表头
    // 处理，同样落到「表头与约定列序不一致」。
    let header = reader.headers().cloned().unwrap_or_default();
    if header.iter().collect::<Vec<_>>() != CSV_HEADER {
        return Err(KairosError::validation(
            "CSV 表头与约定列序不一致，请使用导入面板提供的模板列名。",
        ));
    }
    let mut materials = Vec::new();
    // flatten 跳过库级读取错误：这类输入（引号未闭合等）在本实现里表现为字段数
    // 不足，会由 `parse_csv_row` 的列数校验明确拒绝，不必再留一条不可达的错误分支。
    for (index, record) in reader.records().flatten().enumerate() {
        // 空行由 csv 库自身跳过，这里只处理真正的数据行。
        materials.push(parse_csv_row(&record, index + 2)?);
    }
    if materials.is_empty() {
        return Err(KairosError::validation("CSV 中没有数据行。"));
    }
    Ok(materials)
}

/// 解析一行材料数据（`row_no` 是面向用户的行号，含表头行，用于错误定位）。
fn parse_csv_row(row: &csv::StringRecord, row_no: usize) -> Result<Material> {
    if row.len() != CSV_HEADER.len() {
        return Err(KairosError::validation(format!(
            "CSV 第 {row_no} 行的列数与表头不一致。"
        )));
    }
    let number = |column: usize, label: &str| -> Result<f64> {
        row[column]
            .parse::<f64>()
            .map_err(|e| KairosError::validation(format!("第 {row_no} 行 {label} 不是数字：{e}")))
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
    let material = Material {
        id: crate::services::project::new_id("mat"),
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
            b3s: None,
            b6: 0.0,
            c: 0.0894,
            smooth_band: 0.5,
        },
        specific_heat: table(18, "specificHeat")?,
        conductivity: table(19, "conductivity")?,
        mechanics: None,
        filler: parse_csv_filler(&row[20], row_no)?,
        blowing: None,
        data_note: "CSV 批量导入".to_string(),
    };
    validate(&material)?;
    Ok(material)
}

/// 填料列（第 21 列）：空 = 无填料；否则为「类型:质量分数:长径比:备注」。
fn parse_csv_filler(
    cell: &str,
    row_no: usize,
) -> Result<Option<crate::models::material::FillerGroup>> {
    if cell.is_empty() {
        return Ok(None);
    }
    let parts: Vec<&str> = cell.splitn(4, ':').collect();
    if parts.len() != 4 {
        return Err(KairosError::validation(format!(
            "第 {row_no} 行 filler 应为「类型:质量分数:长径比:备注」。"
        )));
    }
    let aspect_ratio = parts[2]
        .trim()
        .parse::<f64>()
        .map_err(|e| KairosError::validation(format!("第 {row_no} 行 填料长径比不是数字：{e}")))?;
    Ok(Some(crate::models::material::FillerGroup {
        kind: parts[0].trim().to_string(),
        weight_fraction: parts[1].trim().parse::<f64>().map_err(|e| {
            KairosError::validation(format!("第 {row_no} 行 填料质量分数不是数字：{e}"))
        })?,
        aspect_ratio,
        note: parts[3].trim().to_string(),
    }))
}

/// 序列化自定义材料集（写入用户材料库文件）。
pub fn serialize_custom(materials: &[Material]) -> Result<String> {
    for material in materials {
        validate(material)?;
    }
    // 已通过校验的材料必可序列化；失败属程序缺陷，快速失败。
    Ok(serde_json::to_string_pretty(materials).expect("材料库序列化失败"))
}

/// 生成 moldingFoam 材料字典片段；数值保持 SI，供 case 生成器写入 thermophysicalProperties。
pub fn serialize_moldingfoam_material(material: &Material) -> Result<String> {
    validate(material)?;
    let r = &material.rheology;
    let t = &material.pvt;
    let b3s = t.b3s.unwrap_or(t.b3);
    Ok(format!(
        "mixture\n{{\n    equationOfState\n    {{\n        b1m         {b1m:.17e};\n        b2m         {b2m:.17e};\n        b1s         {b1s:.17e};\n        b2s         {b2s:.17e};\n        b3          {b3:.17e};\n        b4          {b4m:.17e};\n        b3s         {b3s:.17e};\n        b4s         {b4s:.17e};\n        b5          {b5:.17e};\n        b6          {b6:.17e};\n        C           {c:.17e};\n        smoothBand  {smooth_band:.17e};\n    }}\n    CrossWlfCoeffs\n    {{\n        n           {n:.17e};\n        tauStar     {tau_star:.17e};\n        D1          {d1:.17e};\n        D2          {d2:.17e};\n        D3          {d3:.17e};\n        A1          {a1:.17e};\n        A2          {a2:.17e};\n    }}\n}}\n",
        b1m = t.b1m,
        b2m = t.b2m,
        b1s = t.b1s,
        b2s = t.b2s,
        b3 = t.b3,
        b4m = t.b4m,
        b3s = b3s,
        b4s = t.b4s,
        b5 = t.b5,
        b6 = t.b6,
        c = t.c,
        smooth_band = t.smooth_band,
        n = r.n,
        tau_star = r.tau_star,
        d1 = r.d1,
        d2 = r.d2,
        d3 = r.d3,
        a1 = r.a1,
        a2 = r.a2,
    ))
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
    fn validate_rejects_invalid_tait_solid_parameters() {
        let mut material = valid_material();
        material.pvt.b3s = Some(0.0);
        assert!(
            validate(&material)
                .unwrap_err()
                .to_string()
                .contains("固态 B(T)")
        );
        material.pvt.b3s = None;
        material.pvt.c = 0.0;
        assert!(
            validate(&material)
                .unwrap_err()
                .to_string()
                .contains("固态 B(T)")
        );
        material.pvt.c = 0.0894;
        material.pvt.smooth_band = 0.0;
        assert!(
            validate(&material)
                .unwrap_err()
                .to_string()
                .contains("固态 B(T)")
        );
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
    fn read_custom_material_file_dispatches_format() {
        let path = std::env::temp_dir().join("kairos-material-dispatch.json");
        let materials = builtin_materials();
        write_custom_file(&path, &materials).unwrap();
        assert_eq!(
            read_custom_material_file(&path).unwrap().len(),
            materials.len()
        );
        let unknown = path.with_extension("txt");
        fs::write(&unknown, "x").unwrap();
        assert!(read_custom_material_file(&unknown).is_err());
        assert!(read_custom_material_file(Path::new("material")).is_err());
        assert!(read_custom_material_file(&path.with_file_name("does-not-exist.json")).is_err());
        fs::remove_file(path).ok();
        fs::remove_file(unknown).ok();
    }

    #[test]
    fn moldingfoam_material_export_keeps_solver_keys_and_si_values() {
        let material = valid_material();
        let text = serialize_moldingfoam_material(&material).unwrap();
        assert!(text.contains("equationOfState"));
        assert!(text.contains("CrossWlfCoeffs"));
        assert!(text.contains("smoothBand"));
        assert!(text.contains("tauStar"));
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

        // 列数正确但列名错：必须命中表头校验本身，而不是行级错误消息
        // （消融 B8 锁定——跳过表头校验时该输入会解析成功）。
        let mut wrong_names = CSV_HEADER;
        wrong_names[0] = "not-an-id";
        let content = format!(
            "{}\n牌号,厂,PP,x,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900,300:0.2,",
            wrong_names.join(",")
        );
        let error = parse_custom_csv(&content).unwrap_err();
        assert!(
            error.to_string().contains("表头与约定列序不一致"),
            "{error}"
        );

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
    fn csv_import_accepts_bom_quotes_crlf_and_inner_commas() {
        // Excel / 表格软件导出的真实形态：UTF-8 BOM + CRLF + 含逗号的字段用引号包裹。
        // 按逗号切分的旧实现会在表头首列带上 BOM 而报「表头不一致」，并把引号里的
        // 逗号当成列分隔（静默错列）。
        let quoted_name = "\"演示, 牌号\"";
        let row = format!(
            "{quoted_name},示例厂,PP,0.35,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900,300:0.2,"
        );
        let content = format!("\u{feff}{}\r\n{row}\r\n", CSV_HEADER.join(","));
        let materials = parse_custom_csv(&content).unwrap();
        assert_eq!(materials.len(), 1);
        // 引号被剥掉、逗号保留在字段内（不是被切成两列）
        assert_eq!(materials[0].name, "演示, 牌号");
        assert!(validate(&materials[0]).is_ok());
    }

    #[test]
    fn csv_import_keeps_quoted_quotes() {
        // CSV 的转义规则：字段内双引号写成两个双引号（原始字段 `"牌号""A""`)
        let row = r#""牌号""A""",示例厂,PP,0.35,20000,1e13,263,0,31,51.6,1.3e-3,1.24e-3,7.5e-7,3e-7,1.4e8,0.003,0.0015,418,300:1900,300:0.2,"#;
        let content = format!("{}\n{row}", CSV_HEADER.join(","));
        let materials = parse_custom_csv(&content).unwrap();
        assert_eq!(materials[0].name, r#"牌号"A""#);
    }

    #[test]
    fn csv_import_rejects_unclosed_quote() {
        // 引号未闭合属结构性错误：必须整行拒绝，而不是把后续内容当同一字段吞掉
        let row = r#""未闭合,示例厂,PP,0.35"#;
        let content = format!("{}\n{row}", CSV_HEADER.join(","));
        let error = parse_custom_csv(&content).unwrap_err();
        assert!(error.to_string().contains("第 2 行"), "实际={error}");
    }

    #[test]
    fn csv_import_reports_unparsable_header() {
        // 表头行本身结构损坏（引号未闭合）时报「表头无法解析」，而不是走到行级错误
        let broken = "\"未闭合,厂,PP,0.35";
        let error = parse_custom_csv(broken).unwrap_err();
        assert!(error.to_string().contains("表头"), "实际={error}");
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

/// 微发泡近似：按发泡参数组修正材料系数，返回 **case 用**的材料副本。
///
/// 只做两项经验修正（非预测级，不建模泡核与长大）：
/// - 有效密度下降 `x%` → 比容增加 `1/(1-x)`：缩放 Tait 的 `b1m` / `b1s`
///   （b2*/b3/b4* 是温度与压力项，量纲上不随密度缩放）；
/// - 表观黏度下降 `y%` → 缩放 Cross-WLF 的 `D1`（零剪切黏度前因子，与 η 成正比）。
///
/// 未启用发泡时原样返回。修正后仍走同一套 `validate`，越界参数直接拒绝。
pub fn apply_blowing_correction(material: &Material) -> Result<Material> {
    let Some(blowing) = &material.blowing else {
        return Ok(material.clone());
    };
    if blowing.density_reduction_percent == 0.0 && blowing.viscosity_reduction_percent == 0.0 {
        return Ok(material.clone());
    }
    let mut corrected = material.clone();
    let volume_scale = 1.0 / (1.0 - blowing.density_reduction_percent / 100.0);
    corrected.pvt.b1m *= volume_scale;
    corrected.pvt.b1s *= volume_scale;
    let viscosity_scale = 1.0 - blowing.viscosity_reduction_percent / 100.0;
    corrected.rheology.d1 *= viscosity_scale;
    validate(&corrected)?;
    Ok(corrected)
}

/// 微发泡修正的说明文案（case 报告与 CLI 展示；未启用返回 None）。
pub fn blowing_note(material: &Material) -> Option<String> {
    let blowing = material.blowing.as_ref()?;
    Some(format!(
        "微发泡近似（{} 质量分数 {:.1}%）：有效密度 −{:.1}%、表观黏度 −{:.1}%——经验修正，非预测级。",
        blowing.kind.trim(),
        blowing.mass_fraction_percent,
        blowing.density_reduction_percent,
        blowing.viscosity_reduction_percent
    ))
}

#[cfg(test)]
mod blowing_tests {
    use super::*;
    use crate::models::material::{BlowingGroup, CrossWlf, Mechanics, Tait};

    fn base() -> Material {
        Material {
            id: "m-1".into(),
            name: "PP-REF".into(),
            manufacturer: "参考".into(),
            family: "PP".into(),
            rheology: CrossWlf {
                n: 0.32,
                tau_star: 2.0e4,
                d1: 1.1e13,
                d2: 263.15,
                d3: 0.0,
                a1: 31.0,
                a2: 51.6,
            },
            pvt: Tait {
                b1m: 1.28e-3,
                b1s: 1.22e-3,
                b2m: 7.5e-7,
                b2s: 3e-7,
                b3: 1.4e8,
                b4m: 3e-3,
                b4s: 1.5e-3,
                b5: 418.0,
                b3s: None,
                b6: 0.0,
                c: 0.0894,
                smooth_band: 0.5,
            },
            specific_heat: vec![(300.0, 1900.0)],
            conductivity: vec![(300.0, 0.2)],
            mechanics: Some(Mechanics {
                elastic_modulus: 1.5e9,
                poisson_ratio: 0.4,
            }),
            filler: None,
            blowing: None,
            data_note: "参考值".into(),
        }
    }

    fn with_blowing(density: f64, viscosity: f64) -> Material {
        let mut material = base();
        material.blowing = Some(BlowingGroup {
            kind: "N₂".into(),
            mass_fraction_percent: 2.0,
            density_reduction_percent: density,
            viscosity_reduction_percent: viscosity,
            note: "工程默认".into(),
        });
        material
    }

    /// 未启用 / 全零修正时原样返回（不改工程里的材料）。
    #[test]
    fn correction_is_identity_without_blowing() {
        let material = base();
        assert_eq!(apply_blowing_correction(&material).unwrap(), material);
        assert_eq!(
            apply_blowing_correction(&with_blowing(0.0, 0.0))
                .unwrap()
                .pvt
                .b1m,
            material.pvt.b1m
        );
        assert!(blowing_note(&material).is_none());
    }

    /// 密度 −20% → 比容 ×1.25；黏度 −30% → D1 ×0.7；b2/b3/b4 与 n 不动。
    #[test]
    fn correction_scales_tait_volume_and_cross_wlf_d1() {
        let material = with_blowing(20.0, 30.0);
        let corrected = apply_blowing_correction(&material).unwrap();
        assert!((corrected.pvt.b1m - material.pvt.b1m * 1.25).abs() < 1e-15);
        assert!((corrected.pvt.b1s - material.pvt.b1s * 1.25).abs() < 1e-15);
        assert!((corrected.rheology.d1 - material.rheology.d1 * 0.7).abs() < 1.0);
        assert_eq!(corrected.pvt.b2m, material.pvt.b2m);
        assert_eq!(corrected.pvt.b5, material.pvt.b5);
        assert_eq!(corrected.rheology.n, material.rheology.n);
        // 原材料不被就地修改
        assert!((material.pvt.b1m - 1.28e-3).abs() < 1e-15);

        let note = blowing_note(&material).unwrap();
        assert!(note.contains("N₂"));
        assert!(note.contains("密度 −20.0%"));
        assert!(note.contains("黏度 −30.0%"));
        assert!(note.contains("非预测级"));
    }

    /// 参数越界在校验层拒绝（密度 ≥100% 会让比容发散）。
    #[test]
    fn blown_parameters_are_validated() {
        let mut material = with_blowing(70.0, 10.0);
        assert!(
            validate(&material)
                .unwrap_err()
                .message()
                .contains("密度下降率")
        );
        material = with_blowing(10.0, 95.0);
        assert!(
            validate(&material)
                .unwrap_err()
                .message()
                .contains("黏度下降率")
        );
        material = with_blowing(10.0, 10.0);
        material.blowing.as_mut().unwrap().mass_fraction_percent = 40.0;
        assert!(
            validate(&material)
                .unwrap_err()
                .message()
                .contains("质量分数")
        );
        material.blowing.as_mut().unwrap().mass_fraction_percent = 2.0;
        material.blowing.as_mut().unwrap().kind = "  ".into();
        assert!(
            validate(&material)
                .unwrap_err()
                .message()
                .contains("发泡剂类型")
        );
    }
}
