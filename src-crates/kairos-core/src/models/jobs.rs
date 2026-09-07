//! 求解作业模型：生命周期状态机 + 并发策略。

use serde::{Deserialize, Serialize};

/// 作业状态：排队 → 运行 → 完成/失败；排队与运行均可取消。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobStatus {
    /// 迁移合法性：状态机的唯一权威定义。
    pub fn can_transition_to(self, next: JobStatus) -> bool {
        use JobStatus::*;
        matches!(
            (self, next),
            (Queued, Running)
                | (Queued, Cancelled)
                | (Running, Done)
                | (Running, Failed)
                | (Running, Cancelled)
        )
    }
}

/// 一个求解作业。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    /// 关联研究（可空：手工提交的冒烟作业）。
    pub study_id: Option<String>,
    pub case_dir: String,
    /// 该作业使用的并行核数（并发预算以核数为单位）。
    pub cores: u32,
    pub status: JobStatus,
    pub created_ms: u64,
    pub started_ms: Option<u64>,
    pub finished_ms: Option<u64>,
    /// 最近一次解析到的求解物理时间（s）。
    pub last_time_s: Option<f64>,
    pub message: Option<String>,
}

impl Job {
    pub fn new(
        id: String,
        study_id: Option<String>,
        case_dir: String,
        cores: u32,
        now_ms: u64,
    ) -> Self {
        Self {
            id,
            study_id,
            case_dir,
            cores,
            status: JobStatus::Queued,
            created_ms: now_ms,
            started_ms: None,
            finished_ms: None,
            last_time_s: None,
            message: None,
        }
    }

    /// 合法迁移：仅当 allowed 时应用；否则返回 None（调用方报错）。
    pub fn transition(&mut self, next: JobStatus, now_ms: u64) -> Option<()> {
        if !self.status.can_transition_to(next) {
            return None;
        }
        self.status = next;
        match next {
            JobStatus::Running => self.started_ms = Some(now_ms),
            JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled => {
                self.finished_ms = Some(now_ms)
            }
            _ => {}
        }
        Some(())
    }
}
