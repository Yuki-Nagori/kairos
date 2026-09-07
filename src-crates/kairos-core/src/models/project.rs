//! 仿真项目模型：Project → Study 两层结构，随 `.kairos` 工程文件持久化。

use serde::{Deserialize, Serialize};

use crate::models::runners::{CoolingChannel, RunnerElement};

/// 当前工程文件 schema 版本：不兼容变更时递增，并在 `services::project::parse` 补迁移。
/// v1→v2：Study 新增流道 / 浇口与冷却水路字段（serde default 迁移，旧文件补空集合）。
pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub studies: Vec<Study>,
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
        }
    }

    /// 添加研究：名称去空白、非空、项目内唯一。
    pub fn add_study(&mut self, id: String, name: &str, now_ms: u64) -> Result<Study, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("研究名称不能为空。".into());
        }
        if self.studies.iter().any(|s| s.name == name) {
            return Err(format!("已存在同名研究：{name}"));
        }
        let study = Study {
            id,
            name: name.to_string(),
            created_ms: now_ms,
            runner_elements: Vec::new(),
            cooling_channels: Vec::new(),
        };
        self.studies.push(study.clone());
        self.updated_ms = now_ms;
        Ok(study)
    }

    pub fn remove_study(&mut self, study_id: &str, now_ms: u64) -> Result<(), String> {
        let before = self.studies.len();
        self.studies.retain(|s| s.id != study_id);
        if self.studies.len() == before {
            return Err(format!("研究不存在：{study_id}"));
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
}
