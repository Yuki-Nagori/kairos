//! 工程文件持久化：`.kairos` = 带版本号的 JSON，原子写入防损坏。

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{KairosError, Result};
use crate::models::project::{Project, RecentProject, SCHEMA_VERSION};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 会话内唯一 ID（进程内自增 + 毫秒时间戳）。
pub fn new_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{:x}-{}",
        now_ms(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// 创建项目：名称去空白、非空。
pub fn create(name: &str, now: u64) -> Result<Project> {
    let name = name.trim();
    let project = Project::new(new_id("proj"), name.to_string(), now);
    validate(&project)?;
    Ok(project)
}

/// 保存前的一致性校验（名称非空、研究名唯一、schema 版本正确）。
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
    for study in &project.studies {
        if !names.insert(study.name.as_str()) {
            return Err(KairosError::validation(format!(
                "研究名称重复：{}",
                study.name
            )));
        }
    }
    Ok(())
}

/// 序列化为工程文件内容。
pub fn serialize(project: &Project) -> Result<String> {
    validate(project)?;
    serde_json::to_string_pretty(project)
        .map_err(|e| KairosError::internal(format!("工程序列化失败：{e}")))
}

/// 解析工程文件内容；高于当前版本的 schema 明确拒绝，旧版本逐级迁移。
/// v1 → v2：Study 新增集合字段；v2 → v3：Study 新增 process；v3 → v4：Study 新增 material_id
/// （均为 `serde(default)` 补默认）。
pub fn parse(content: &str) -> Result<Project> {
    let mut project: Project = serde_json::from_str(content)
        .map_err(|e| KairosError::validation(format!("工程文件无法解析：{e}")))?;
    match project.schema_version.cmp(&SCHEMA_VERSION) {
        std::cmp::Ordering::Equal => Ok(project),
        std::cmp::Ordering::Greater => Err(KairosError::validation(format!(
            "工程文件版本过新（{} > {SCHEMA_VERSION}），请升级 Kairos。",
            project.schema_version
        ))),
        std::cmp::Ordering::Less => {
            // v1 → v2 的字段补空已由 serde(default) 完成；未来新版本在此追加迁移步骤。
            project.schema_version = SCHEMA_VERSION;
            validate(&project)?;
            Ok(project)
        }
    }
}

/// 原子写入：先写同目录临时文件再改名覆盖；Windows 的 rename 不能覆盖既有目标，先移除。
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("kairos.tmp");
    fs::write(&tmp, content).map_err(|e| KairosError::io(format!("写入临时文件失败：{e}")))?;
    #[cfg(target_os = "windows")]
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        KairosError::io(format!("工程文件替换失败：{e}"))
    })
}

/// 解析最近项目 JSON；损坏或缺失按空列表处理（非关键数据，不做硬失败）。
pub fn parse_recents(content: &str) -> Vec<RecentProject> {
    serde_json::from_str(content).unwrap_or_default()
}

pub fn serialize_recents(recents: &[RecentProject]) -> Result<String> {
    serde_json::to_string_pretty(recents)
        .map_err(|e| KairosError::internal(format!("最近项目序列化失败：{e}")))
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
        let parsed = parse_recents(&serialize_recents(&recents).unwrap());
        assert_eq!(parsed, recents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Project {
        let mut project = create("演示项目", 1000).unwrap();
        project.add_study("s-1".into(), "填充分析", 1001).unwrap();
        project
    }

    #[test]
    fn create_rejects_blank_name() {
        assert!(create("   ", 1000).is_err());
        assert_eq!(create(" A ", 1000).unwrap().name, "A");
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(new_id("x"), new_id("x"));
    }

    #[test]
    fn serialize_parse_roundtrip() {
        let project = sample();
        let parsed = parse(&serialize(&project).unwrap()).unwrap();
        assert_eq!(parsed, project);
    }

    #[test]
    fn parse_rejects_newer_schema_and_garbage() {
        let newer = serde_json::json!({ "schemaVersion": 999, "id": "p", "name": "n", "createdMs": 1, "updatedMs": 1, "studies": [] });
        let error = parse(&newer.to_string()).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);

        assert!(parse("not json").is_err());
    }

    #[test]
    fn v1_project_migrates_to_current_schema() {
        let v1 = r#"{"schemaVersion":1,"id":"p-1","name":"旧工程","createdMs":1,"updatedMs":1,"studies":[{"id":"s-1","name":"填充","createdMs":2}]}"#;
        let project = parse(v1).unwrap();
        assert_eq!(project.schema_version, SCHEMA_VERSION);
        assert_eq!(project.studies[0].runner_elements.len(), 0);
        assert_eq!(project.studies[0].cooling_channels.len(), 0);
        assert!(project.studies[0].process.is_none());
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
}
