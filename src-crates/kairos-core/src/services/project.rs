//! 工程文件持久化：`.kairos` = 带版本号的 JSON，原子写入防损坏。

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{KairosError, Result};
use crate::models::project::{Project, RecentProject, SCHEMA_VERSION, Study};

/// 当前 Unix 毫秒时间戳；实现在 [`crate::utils::time`]（同类语义只留一处）。
pub use crate::utils::time::now_ms;

/// 会话内唯一 ID（进程内自增 + 毫秒时间戳）。
pub fn new_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{:x}-{}",
        now_ms(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// 新工程的默认方案名。
pub const DEFAULT_STUDY_NAME: &str = "方案 1";

/// 创建项目：名称去空白、非空；新工程自带一个默认方案。
///
/// 方案是材料 / 工艺 / 浇注系统 / 求解任务的承载单元，没有方案的空工程在界面上
/// 走不到任何分析步骤，因此创建即给一个（旧文件里没有方案时由前端补默认选中）。
pub fn create(name: &str, now: u64) -> Result<Project> {
    let name = name.trim();
    let mut project = Project::new(new_id("proj"), name.to_string(), now);
    add_study(&mut project, new_id("study"), DEFAULT_STUDY_NAME, now)?;
    validate(&project)?;
    Ok(project)
}

/// 添加方案：名称去空白、非空、项目内唯一。
///
/// 规则住 services 而非 DTO——`models` 只描述持久化形状，判断与错误类别属领域逻辑。
pub fn add_study(project: &mut Project, id: String, name: &str, now: u64) -> Result<Study> {
    let name = name.trim();
    if name.is_empty() {
        return Err(KairosError::validation("方案名称不能为空。"));
    }
    if project.studies.iter().any(|study| study.name == name) {
        return Err(KairosError::validation(format!("已存在同名方案：{name}")));
    }
    let study = Study {
        id,
        name: name.to_string(),
        created_ms: now,
        runner_elements: Vec::new(),
        cooling_channels: Vec::new(),
        process: None,
        material_id: None,
    };
    project.studies.push(study.clone());
    project.updated_ms = now;
    Ok(study)
}

/// 移除方案；不存在时报 `not_found`（前端据此区分「已被删掉」与其它失败）。
pub fn remove_study(project: &mut Project, study_id: &str, now: u64) -> Result<()> {
    let before = project.studies.len();
    project.studies.retain(|study| study.id != study_id);
    if project.studies.len() == before {
        return Err(KairosError::not_found(format!("方案不存在：{study_id}")));
    }
    project.updated_ms = now;
    Ok(())
}

/// 保存前的一致性校验（名称非空、方案名唯一、schema 版本正确）。
pub fn validate(project: &Project) -> Result<()> {
    if project.name.trim().is_empty() {
        return Err(KairosError::validation("项目名称不能为空。"));
    }
    if project.schema_version != SCHEMA_VERSION {
        return Err(KairosError::internal(format!(
            "项目 schema 版本异常：{}（期望 {SCHEMA_VERSION}）",
            project.schema_version
        )));
    }
    let mut names = HashSet::new();
    let mut ids = HashSet::new();
    for study in &project.studies {
        crate::services::paths::validate_id(&study.id)?;
        if !ids.insert(&study.id) {
            return Err(KairosError::validation("方案 ID 重复。"));
        }
        if !names.insert(study.name.as_str()) {
            return Err(KairosError::validation(format!(
                "方案名称重复：{}",
                study.name
            )));
        }
    }
    Ok(())
}

/// 序列化为工程文件内容。
pub fn serialize(project: &Project) -> Result<String> {
    validate(project)?;
    // 已通过校验的工程数据必可序列化；失败属程序缺陷，快速失败。
    Ok(serde_json::to_string_pretty(project).expect("工程序列化失败"))
}

/// 解析工程文件内容；仅接受当前版本 schema，过新 / 过旧版本明确拒绝。
pub fn parse(content: &str) -> Result<Project> {
    let project: Project = serde_json::from_str(content)
        .map_err(|e| KairosError::validation(format!("工程文件无法解析：{e}")))?;
    match project.schema_version.cmp(&SCHEMA_VERSION) {
        std::cmp::Ordering::Equal => {
            validate(&project)?;
            Ok(project)
        }
        std::cmp::Ordering::Greater => Err(KairosError::validation(format!(
            "工程文件版本过新（{} > {SCHEMA_VERSION}），请升级 Kairos。",
            project.schema_version
        ))),
        std::cmp::Ordering::Less => Err(KairosError::validation(format!(
            "工程文件版本过旧（{} < {SCHEMA_VERSION}），已不支持打开。",
            project.schema_version
        ))),
    }
}

/// 原子写入工程文件：实现在 [`crate::utils::fs::write_atomic`]。
///
/// 保留本入口是为了让 `services::project` 的使用方（材料库、适配层、CLI）不必各自
/// 拼动作描述——`what` 固定为「工程文件」。
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    crate::utils::fs::write_atomic(path, content, "工程文件")
}

/// 解析最近项目 JSON；损坏或缺失按空列表处理（非关键数据，不做硬失败）。
pub fn parse_recents(content: &str) -> Vec<RecentProject> {
    serde_json::from_str(content).unwrap_or_default()
}

pub fn serialize_recents(recents: &[RecentProject]) -> String {
    // 纯字符串结构的序列化不会失败；失败属程序缺陷，快速失败。
    serde_json::to_string_pretty(recents).expect("最近项目序列化失败")
}

/// 登记最近项目：同名路径去重置顶、最多保留 10 条。
pub fn record_recent(
    mut recents: Vec<RecentProject>,
    path: String,
    name: String,
    now: u64,
) -> Vec<RecentProject> {
    recents.retain(|recent| recent.path != path);
    recents.insert(
        0,
        RecentProject {
            path,
            name,
            last_opened_ms: now,
        },
    );
    recents.truncate(10);
    recents
}

#[cfg(test)]
mod recents_tests {
    use super::*;
    use crate::models::project::RecentProject;

    #[test]
    fn parse_recents_tolerates_garbage() {
        assert!(parse_recents("not json").is_empty());
        assert!(parse_recents("[]").is_empty());
    }

    #[test]
    fn record_recent_dedupes_top_and_caps_at_ten() {
        let entry = |n: u64| RecentProject {
            path: format!("/p/{n}"),
            name: format!("项目{n}"),
            last_opened_ms: n,
        };
        let recents: Vec<RecentProject> = (0..12).map(entry).collect();
        let updated = record_recent(recents, "/p/5".into(), "五".into(), 999);
        assert_eq!(updated.len(), 10);
        assert_eq!(updated[0].path, "/p/5");
        assert_eq!(updated[0].name, "五");
        assert!(updated[1..].iter().all(|r| r.path != "/p/5"));
    }

    #[test]
    fn recents_roundtrip() {
        let recents = record_recent(Vec::new(), "/p/1".into(), "一".into(), 1);
        let parsed = parse_recents(&serialize_recents(&recents));
        assert_eq!(parsed, recents);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsing_rejects_unsafe_and_duplicate_study_ids() {
        let mut project = super::create("test", 0).unwrap();
        project.studies[0].id = "../outside".into();
        let json = serde_json::to_string(&project).unwrap();
        assert!(super::parse(&json).is_err());
        project.studies[0].id = "safe".into();
        let mut second = project.studies[0].clone();
        second.name = "second".into();
        project.studies.push(second);
        assert!(
            super::validate(&project)
                .unwrap_err()
                .message()
                .contains("ID 重复")
        );
    }

    use super::*;
    use crate::models::process::ProcessSettings;
    use std::fs;

    fn sample() -> Project {
        let mut project = create("演示项目", 1000).unwrap();
        add_study(&mut project, "s-1".into(), "填充分析", 1001).unwrap();
        // 方案配置（材料 / 工艺 / 杆系 / 水路）与几何引用都在工程文件里：
        // 样本带上它们，序列化往返才锁得住这些字段（少一个就会静默丢配置）。
        let study = &mut project.studies[0];
        study.material_id = Some("builtin-pp-001".into());
        study.process = Some(ProcessSettings {
            melt_temp_c: 230.0,
            mold_temp_c: 40.0,
            ejection_temp_c: 90.0,
            injection_time_s: 1.5,
            vp_switch_volume_percent: 96.0,
            packing_pressure_mpa_curve: vec![(0.0, 60.0), (8.0, 48.0)],
            packing_time_s: 8.0,
            cooling_time_s: 15.0,
            coolant_temp_c: 25.0,
        });
        study
            .runner_elements
            .push(crate::models::runners::RunnerElement {
                id: "re-1".into(),
                kind: crate::models::runners::RunnerKind::Gate,
                diameter_mm: 6.0,
                start: [0.0, 0.0, 0.0],
                end: [0.0, 0.0, 1.0],
                medium: None,
            });
        project.upsert_geometry(
            crate::models::project::GeometryRef {
                id: "geo-1".into(),
                file_name: "part.stl".into(),
                relative_path: "geometry/part.stl".into(),
            },
            1002,
        );
        project
    }

    #[test]
    fn create_rejects_blank_name() {
        assert!(create("   ", 1000).is_err());
        assert_eq!(create(" A ", 1000).unwrap().name, "A");
    }

    #[test]
    fn create_seeds_default_study() {
        let project = create("演示项目", 1000).unwrap();
        assert_eq!(project.studies.len(), 1);
        assert_eq!(project.studies[0].name, DEFAULT_STUDY_NAME);
        assert_eq!(project.studies[0].created_ms, 1000);
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(new_id("x"), new_id("x"));
    }

    #[test]
    fn gas_runner_medium_survives_the_roundtrip() {
        // 注气通道的数据位：介质标记必须随工程文件往返保真（气体辅助注塑的第一步，
        // 气体相定义与入口边界属上游能力就绪后的第二步）。
        let mut project = sample();
        project.studies[0].runner_elements[0].medium =
            Some(crate::models::runners::RunnerMedium::Gas);
        let parsed = parse(&serialize(&project).unwrap()).unwrap();
        assert_eq!(
            parsed.studies[0].runner_elements[0].medium,
            Some(crate::models::runners::RunnerMedium::Gas)
        );
        // 旧工程文件没有该字段：解析为 None（＝熔体），不报错
        let json = serialize(&sample())
            .unwrap()
            .replace(", \"medium\": null", "");
        let parsed = parse(&json).unwrap();
        assert_eq!(parsed.studies[0].runner_elements[0].medium, None);
    }

    #[test]
    fn serialize_parse_roundtrip() {
        let project = sample();
        let parsed = parse(&serialize(&project).unwrap()).unwrap();
        assert_eq!(parsed, project);
    }

    #[test]
    fn add_study_trims_name_and_touches_updated_ms() {
        let mut project = create("演示项目", 1000).unwrap();
        let study = add_study(&mut project, "s-2".into(), " 填充分析 ", 1001).unwrap();
        assert_eq!(study.name, "填充分析");
        assert_eq!(project.studies[1].name, "填充分析");
        assert_eq!(project.updated_ms, 1001);
    }

    #[test]
    fn add_study_rejects_blank_and_duplicate_names() {
        let mut project = create("演示项目", 1000).unwrap();
        let blank = add_study(&mut project, "s-2".into(), "   ", 1001).unwrap_err();
        assert_eq!(blank.kind(), crate::error::ErrorKind::Validation);
        // create 已建了「方案 1」，同名再建即冲突
        let duplicate =
            add_study(&mut project, "s-3".into(), DEFAULT_STUDY_NAME, 1002).unwrap_err();
        assert_eq!(duplicate.kind(), crate::error::ErrorKind::Validation);
        assert!(duplicate.message().contains("已存在同名方案"));
    }

    #[test]
    fn remove_study_reports_missing_as_not_found() {
        let mut project = create("演示项目", 1000).unwrap();
        // create 已播下默认方案，取其真实 id（生成的 id 不是固定值）
        let seeded = project.studies[0].id.clone();
        remove_study(&mut project, &seeded, 1002).unwrap();
        assert!(project.studies.is_empty());
        assert_eq!(project.updated_ms, 1002);

        let missing = remove_study(&mut project, &seeded, 1003).unwrap_err();
        assert_eq!(missing.kind(), crate::error::ErrorKind::NotFound);
        assert!(missing.message().contains("方案不存在"));
    }

    #[test]
    fn parse_rejects_newer_schema_and_garbage() {
        let newer = serde_json::json!({ "schemaVersion": 999, "id": "p", "name": "n", "createdMs": 1, "updatedMs": 1, "studies": [] });
        let error = parse(&newer.to_string()).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);

        assert!(parse("not json").is_err());
    }

    #[test]
    fn v1_schema_is_rejected() {
        let v1 = r#"{"schemaVersion":1,"id":"p-1","name":"旧工程","createdMs":1,"updatedMs":1,"studies":[{"id":"s-1","name":"填充","createdMs":2}]}"#;
        let error = parse(v1).unwrap_err();
        assert!(error.to_string().contains("版本过旧"));
    }

    #[test]
    fn serialize_validates_before_writing() {
        let mut project = sample();
        project.schema_version = 42;
        assert!(serialize(&project).is_err());
    }

    #[test]
    fn write_atomic_roundtrip() {
        let path = std::env::temp_dir().join(format!("kairos-t03-{}.kairos", now_ms()));
        let project = sample();
        write_atomic(&path, &serialize(&project).unwrap()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(parse(&content).unwrap(), project);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn write_atomic_reports_tmp_write_failure() {
        let dir = std::env::temp_dir().join(format!("kairos-t03b-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("project.kairos");
        // 预埋同名临时文件为目录，令临时文件写入失败。
        fs::create_dir_all(target.with_extension("kairos.tmp")).unwrap();
        let error = write_atomic(&target, "内容").unwrap_err();
        assert!(error.to_string().contains("写入临时文件失败"));
        fs::remove_dir_all(&dir).ok();
    }
}
