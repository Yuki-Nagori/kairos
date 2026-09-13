//! 仿真项目模型：Project → Study 两层结构，随 `.kairos` 工程文件持久化。

use serde::{Deserialize, Serialize};

use crate::models::process::ProcessSettings;
use crate::models::runners::{CoolingChannel, RunnerElement};

/// 当前工程文件 schema 版本：不兼容变更时递增，并在 `services::project::parse` 补迁移。
/// v1→v2：Study 新增流道 / 浇口与冷却水路字段（serde default 迁移，旧文件补空集合）。
/// v2→v3：Study 新增工艺设置（Option，serde default 迁移为 None）。
/// v3→v4：Study 新增材料引用 material_id（Option，serde default 迁移为 None）。
/// v4→v5：Project 新增几何引用 geometries（工作区相对路径，serde default 迁移为空集合）。
pub const SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub studies: Vec<Study>,
    /// 工程内的几何引用（工作区相对路径；散装工程为空，几何只活在会话内存里）。
    #[serde(default)]
    pub geometries: Vec<GeometryRef>,
}

/// 工程内的几何引用：文件在工作区 `geometry/` 下的相对路径。
/// 绝对路径不写入工程文件——换机器 / 换盘即失效。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeometryRef {
    /// 会话内几何 id（导入时生成，跨会话稳定地标识同一份几何）。
    pub id: String,
    /// 展示用文件名（归档后的文件名）。
    pub file_name: String,
    /// 相对工作区根的路径，如 `geometry/part.stl`。
    pub relative_path: String,
}

impl Project {
    /// 登记几何引用：同 id 覆盖（重复导入同名文件时更新引用），否则追加。
    pub fn upsert_geometry(&mut self, reference: GeometryRef, now_ms: u64) {
        match self
            .geometries
            .iter_mut()
            .find(|existing| existing.id == reference.id)
        {
            Some(existing) => *existing = reference,
            None => self.geometries.push(reference),
        }
        self.updated_ms = now_ms;
    }

    /// 移除几何引用（几何被移除时同步清理）。
    pub fn remove_geometry(&mut self, geometry_id: &str, now_ms: u64) {
        let before = self.geometries.len();
        self.geometries.retain(|entry| entry.id != geometry_id);
        if self.geometries.len() != before {
            self.updated_ms = now_ms;
        }
    }
}

/// 最近打开的工程记录（适配层持久化在应用数据目录，随命令返回前端）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub last_opened_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Study {
    pub id: String,
    pub name: String,
    pub created_ms: u64,
    /// 流道 / 浇口单元（v2 新增，旧版本文件迁移为空集合）。
    #[serde(default)]
    pub runner_elements: Vec<RunnerElement>,
    /// 冷却水路单元（v2 新增，旧版本文件迁移为空集合）。
    #[serde(default)]
    pub cooling_channels: Vec<CoolingChannel>,
    /// 成型工艺设置（v3 新增，未设置时为 None）。
    #[serde(default)]
    pub process: Option<ProcessSettings>,
    /// 选用的材料 id（v4 新增，未设置时为 None）。
    #[serde(default)]
    pub material_id: Option<String>,
}

impl Project {
    pub fn new(id: String, name: String, now_ms: u64) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id,
            name,
            created_ms: now_ms,
            updated_ms: now_ms,
            studies: Vec::new(),
            geometries: Vec::new(),
        }
    }

    /// 添加方案：名称去空白、非空、项目内唯一。
    pub fn add_study(&mut self, id: String, name: &str, now_ms: u64) -> Result<Study, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("方案名称不能为空。".into());
        }
        if self.studies.iter().any(|s| s.name == name) {
            return Err(format!("已存在同名方案：{name}"));
        }
        let study = Study {
            id,
            name: name.to_string(),
            created_ms: now_ms,
            runner_elements: Vec::new(),
            cooling_channels: Vec::new(),
            process: None,
            material_id: None,
        };
        self.studies.push(study.clone());
        self.updated_ms = now_ms;
        Ok(study)
    }

    pub fn remove_study(&mut self, study_id: &str, now_ms: u64) -> Result<(), String> {
        let before = self.studies.len();
        self.studies.retain(|s| s.id != study_id);
        if self.studies.len() == before {
            return Err(format!("方案不存在：{study_id}"));
        }
        self.updated_ms = now_ms;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project::new("p-1".into(), "演示项目".into(), 1000)
    }

    #[test]
    fn add_study_validates_name() {
        let mut p = project();
        assert!(p.add_study("s-1".into(), "  ", 1001).is_err());
        assert!(p.add_study("s-1".into(), " 填充分析 ", 1001).is_ok());
        assert_eq!(p.studies[0].name, "填充分析");
        assert_eq!(p.updated_ms, 1001);
    }

    #[test]
    fn add_study_rejects_duplicate_names() {
        let mut p = project();
        p.add_study("s-1".into(), "填充", 1001).unwrap();
        assert!(p.add_study("s-2".into(), "填充", 1002).is_err());
    }

    #[test]
    fn remove_study_reports_missing() {
        let mut p = project();
        p.add_study("s-1".into(), "填充", 1001).unwrap();
        p.remove_study("s-1", 1002).unwrap();
        assert!(p.remove_study("s-1", 1003).is_err());
    }

    #[test]
    fn new_project_uses_current_schema() {
        assert_eq!(project().schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn upsert_geometry_replaces_same_id_and_removes() {
        let mut p = project();
        let first = GeometryRef {
            id: "g-1".into(),
            file_name: "part.stl".into(),
            relative_path: "geometry/part.stl".into(),
        };
        p.upsert_geometry(first.clone(), 1001);
        assert_eq!(p.geometries.len(), 1);
        assert_eq!(p.updated_ms, 1001);

        // 同 id 覆盖（重新导入同名文件）
        let updated = GeometryRef {
            file_name: "part-v2.stl".into(),
            relative_path: "geometry/g-1-part-v2.stl".into(),
            ..first
        };
        p.upsert_geometry(updated, 1002);
        assert_eq!(p.geometries.len(), 1);
        assert_eq!(p.geometries[0].file_name, "part-v2.stl");

        // 不同 id 追加；移除按 id
        p.upsert_geometry(
            GeometryRef {
                id: "g-2".into(),
                file_name: "other.stl".into(),
                relative_path: "geometry/other.stl".into(),
            },
            1003,
        );
        assert_eq!(p.geometries.len(), 2);
        p.remove_geometry("g-1", 1004);
        assert_eq!(p.geometries.len(), 1);
        assert_eq!(p.geometries[0].id, "g-2");
        assert_eq!(p.updated_ms, 1004);
        // 不存在的 id：时间戳不变
        p.remove_geometry("ghost", 1005);
        assert_eq!(p.updated_ms, 1004);
    }
}
