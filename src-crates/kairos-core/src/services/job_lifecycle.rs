//! 求解作业运行期状态机。
//!
//! 进程、VM 和 Tauri Channel 都属于适配层；这里仅定义它们向领域层报告的事件。
//! 这样取消、启动失败和远端状态未知不会被某个具体 runner 的控制流“顺带”决定。

use crate::error::{KairosError, Result};

/// runner 当前所处阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Queued,
    Preparing,
    Starting,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

/// 适配层可报告给生命周期服务的最小事件集合。
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    BeginPreparation,
    Prepared,
    Started,
    Progress(f64),
    RequestCancel,
    CancelConfirmed,
    Succeeded,
    Failed(String),
    RemoteUnknown(String),
    RemoteRecovered,
}

/// 可注入 runner 的纯状态机。
#[derive(Debug, Clone, PartialEq)]
pub struct JobLifecycle {
    phase: RunPhase,
    progress_s: Option<f64>,
    message: Option<String>,
}

impl Default for JobLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl JobLifecycle {
    pub fn new() -> Self {
        Self {
            phase: RunPhase::Queued,
            progress_s: None,
            message: None,
        }
    }

    pub fn phase(&self) -> RunPhase {
        self.phase
    }

    pub fn progress_s(&self) -> Option<f64> {
        self.progress_s
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// 应用一个 runner 事件；非法迁移返回错误且不改变当前状态。
    pub fn apply(&mut self, event: RunEvent) -> Result<()> {
        let previous = self.clone();
        let result = self.apply_inner(event);
        if result.is_err() {
            *self = previous;
        }
        result
    }

    fn apply_inner(&mut self, event: RunEvent) -> Result<()> {
        use RunEvent::*;
        use RunPhase::{
            Cancelled, Cancelling, Completed, Failed as FailedPhase, Preparing, Running, Starting,
            Unknown,
        };
        match (self.phase, event) {
            (RunPhase::Queued, BeginPreparation) => self.phase = Preparing,
            (RunPhase::Queued, RequestCancel) => self.phase = Cancelled,
            (Preparing, Prepared) => self.phase = Starting,
            (Preparing, RequestCancel) => self.phase = Cancelled,
            (Starting, Started) => self.phase = Running,
            (Starting, RequestCancel) => self.phase = Cancelling,
            (Running, Progress(value)) if value.is_finite() => self.progress_s = Some(value),
            (Running, RequestCancel) => self.phase = Cancelling,
            (Running, Succeeded) => self.phase = Completed,
            (Running, Failed(message)) => {
                self.phase = FailedPhase;
                self.message = Some(message);
            }
            (Running, RemoteUnknown(message)) => {
                self.phase = Unknown;
                self.message = Some(message);
            }
            (Cancelling, CancelConfirmed) => self.phase = Cancelled,
            (Unknown, RemoteRecovered) => self.phase = Running,
            (Unknown, CancelConfirmed) => self.phase = Cancelled,
            (Unknown, Failed(message)) => {
                self.phase = FailedPhase;
                self.message = Some(message);
            }
            (Preparing, Failed(message)) | (Starting, Failed(message)) => {
                self.phase = FailedPhase;
                self.message = Some(message);
            }
            _ => {
                return Err(KairosError::internal(format!(
                    "作业生命周期非法迁移：{:?}",
                    self.phase
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> JobLifecycle {
        let mut lifecycle = JobLifecycle::new();
        lifecycle.apply(RunEvent::BeginPreparation).unwrap();
        lifecycle.apply(RunEvent::Prepared).unwrap();
        lifecycle.apply(RunEvent::Started).unwrap();
        lifecycle
    }

    #[test]
    fn preparation_cancel_does_not_start_a_runner() {
        let mut lifecycle = JobLifecycle::new();
        lifecycle.apply(RunEvent::BeginPreparation).unwrap();
        lifecycle.apply(RunEvent::RequestCancel).unwrap();
        assert_eq!(lifecycle.phase(), RunPhase::Cancelled);
        assert!(lifecycle.apply(RunEvent::Started).is_err());
    }

    #[test]
    fn queued_cancel_and_start_cancel_are_distinct_paths() {
        let mut queued = JobLifecycle::default();
        queued.apply(RunEvent::RequestCancel).unwrap();
        assert_eq!(queued.phase(), RunPhase::Cancelled);

        let mut starting = JobLifecycle::new();
        starting.apply(RunEvent::BeginPreparation).unwrap();
        starting.apply(RunEvent::Prepared).unwrap();
        starting.apply(RunEvent::RequestCancel).unwrap();
        starting.apply(RunEvent::CancelConfirmed).unwrap();
        assert_eq!(starting.phase(), RunPhase::Cancelled);
    }

    #[test]
    fn start_failure_is_terminal_and_keeps_message() {
        let mut lifecycle = JobLifecycle::new();
        lifecycle.apply(RunEvent::BeginPreparation).unwrap();
        lifecycle
            .apply(RunEvent::Failed("runner 启动失败".into()))
            .unwrap();
        assert_eq!(lifecycle.phase(), RunPhase::Failed);
        assert_eq!(lifecycle.message(), Some("runner 启动失败"));
    }

    #[test]
    fn running_success_and_failure_are_terminal() {
        let mut success = running();
        success.apply(RunEvent::Succeeded).unwrap();
        assert_eq!(success.phase(), RunPhase::Completed);

        let mut failure = running();
        failure
            .apply(RunEvent::Failed("求解器退出".into()))
            .unwrap();
        assert_eq!(failure.phase(), RunPhase::Failed);

        let mut preparing = JobLifecycle::new();
        preparing.apply(RunEvent::BeginPreparation).unwrap();
        preparing
            .apply(RunEvent::Failed("准备失败".into()))
            .unwrap();
        assert_eq!(preparing.phase(), RunPhase::Failed);
    }

    #[test]
    fn unknown_remote_state_must_be_recovered_or_confirmed() {
        let mut lifecycle = running();
        lifecycle
            .apply(RunEvent::RemoteUnknown("状态轮询超时".into()))
            .unwrap();
        assert_eq!(lifecycle.phase(), RunPhase::Unknown);
        assert!(lifecycle.apply(RunEvent::Succeeded).is_err());
        lifecycle.apply(RunEvent::RemoteRecovered).unwrap();
        lifecycle.apply(RunEvent::RequestCancel).unwrap();
        lifecycle.apply(RunEvent::CancelConfirmed).unwrap();
        assert_eq!(lifecycle.phase(), RunPhase::Cancelled);

        let mut failed = running();
        failed
            .apply(RunEvent::RemoteUnknown("断联".into()))
            .unwrap();
        failed.apply(RunEvent::Failed("远端退出".into())).unwrap();
        assert_eq!(failed.phase(), RunPhase::Failed);

        let mut confirmed = running();
        confirmed
            .apply(RunEvent::RemoteUnknown("再次断联".into()))
            .unwrap();
        confirmed.apply(RunEvent::CancelConfirmed).unwrap();
        assert_eq!(confirmed.phase(), RunPhase::Cancelled);
    }

    #[test]
    fn invalid_progress_is_rejected_without_mutating_state() {
        let mut lifecycle = running();
        assert!(lifecycle.apply(RunEvent::Progress(f64::NAN)).is_err());
        assert_eq!(lifecycle.progress_s(), None);
        assert!(lifecycle.apply(RunEvent::Progress(1.25)).is_ok());
        assert_eq!(lifecycle.progress_s(), Some(1.25));
    }

    #[test]
    fn invalid_event_rolls_back_phase_and_message() {
        let mut lifecycle = running();
        lifecycle
            .apply(RunEvent::RemoteUnknown("断联".into()))
            .unwrap();
        assert!(lifecycle.apply(RunEvent::Succeeded).is_err());
        assert_eq!(lifecycle.phase(), RunPhase::Unknown);
        assert_eq!(lifecycle.message(), Some("断联"));
    }
}
