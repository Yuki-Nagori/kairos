//! 作业调度纯逻辑：并发预算（同时运行数 + 总核数）与状态迁移。
//! 进程/线程等副作用由适配层执行，这里只操作 Vec<Job>，方便穷举测试。

use crate::error::Result;
use crate::models::jobs::{Job, JobStatus};

/// 默认调度核数预算；提交与 case 生成共用同一口径。
pub const MAX_JOB_CORES: u32 = 8;

/// 拒绝当前调度器永远无法启动的核数请求。
pub fn validate_cores(cores: u32) -> Result<()> {
    if !(1..=MAX_JOB_CORES).contains(&cores) {
        return Err(crate::error::KairosError::validation(format!(
            "核数必须在 1 ~ {MAX_JOB_CORES} 之间。"
        )));
    }
    Ok(())
}

/// 调度参数：同时运行作业数上限与总核数预算。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchedulerLimits {
    pub max_concurrent: usize,
    pub max_cores: u32,
}

impl SchedulerLimits {
    pub fn new(max_concurrent: usize, max_cores: u32) -> Self {
        Self {
            max_concurrent: max_concurrent.max(1),
            max_cores: max_cores.max(1),
        }
    }
}

fn find(jobs: &[Job], id: &str) -> Result<usize> {
    jobs.iter()
        .position(|job| job.id == id)
        .ok_or_else(|| crate::error::KairosError::not_found(format!("作业不存在：{id}")))
}

/// 运行中的核数占用。
pub fn used_cores(jobs: &[Job]) -> u32 {
    jobs.iter()
        .filter(|job| job.status == JobStatus::Running)
        .map(|job| job.cores)
        .sum()
}

/// 提交作业：进入队列尾部（Queued）。
pub fn submit(
    jobs: &mut Vec<Job>,
    id: String,
    study_id: Option<String>,
    case_dir: String,
    cores: u32,
    now_ms: u64,
) -> Result<()> {
    validate_cores(cores)?;
    jobs.push(Job::new(id, study_id, case_dir, cores, now_ms));
    Ok(())
}

/// 按预算提升排队作业：返回本轮应启动的作业 ID 列表（适配层据此真正起进程）。
/// 已运行作业优先占用核数预算；队列按提交顺序消费。
pub fn promote_ready(jobs: &mut [Job], limits: &SchedulerLimits, now_ms: u64) -> Vec<String> {
    let mut used_cores = used_cores(jobs);
    let mut running = jobs
        .iter()
        .filter(|job| job.status == JobStatus::Running)
        .count();
    let mut started = Vec::new();
    for job in jobs.iter_mut() {
        if running >= limits.max_concurrent {
            break;
        }
        // 仅排队中且不超核数预算的作业才启动；其余保持排队。
        if job.status == JobStatus::Queued && used_cores + job.cores <= limits.max_cores {
            job.status = JobStatus::Running;
            job.started_ms = Some(now_ms);
            used_cores += job.cores;
            running += 1;
            started.push(job.id.clone());
        }
    }
    started
}

/// 迁移到运行中（由适配层在真正起进程前调用，保证状态先行）。
pub fn mark_running(jobs: &mut [Job], id: &str, now_ms: u64) -> Result<()> {
    let job = &mut jobs[find(jobs, id)?];
    job.transition(JobStatus::Running, now_ms)
        .ok_or_else(|| crate::error::KairosError::internal(format!("非法迁移：运行中 {id}")))?;
    Ok(())
}

/// 更新求解物理时间进度。
pub fn update_progress(jobs: &mut [Job], id: &str, time_s: f64) -> Result<()> {
    let job = &mut jobs[find(jobs, id)?];
    if job.status != JobStatus::Running {
        return Err(crate::error::KairosError::internal(
            "仅运行中的作业可更新进度。",
        ));
    }
    job.last_time_s = Some(time_s);
    Ok(())
}

fn finish(
    jobs: &mut [Job],
    id: &str,
    status: JobStatus,
    message: Option<String>,
    now_ms: u64,
) -> Result<()> {
    let job = &mut jobs[find(jobs, id)?];
    job.transition(status, now_ms)
        .ok_or_else(|| crate::error::KairosError::internal(format!("非法迁移：{id}")))?;
    job.message = message;
    Ok(())
}

pub fn mark_done(jobs: &mut [Job], id: &str, now_ms: u64) -> Result<()> {
    finish(jobs, id, JobStatus::Done, None, now_ms)
}

pub fn mark_failed(jobs: &mut [Job], id: &str, message: &str, now_ms: u64) -> Result<()> {
    finish(
        jobs,
        id,
        JobStatus::Failed,
        Some(message.to_string()),
        now_ms,
    )
}

/// 取消：排队中直接取消；运行中的由适配层先终止进程组再调用本函数。
pub fn cancel(jobs: &mut [Job], id: &str, now_ms: u64) -> Result<()> {
    finish(
        jobs,
        id,
        JobStatus::Cancelled,
        Some("用户取消".into()),
        now_ms,
    )
}

/// 作业收尾判定：返回失败原因（`None` = 成功）。
///
/// 求解器错误标记优先提供诊断；脚本保留求解与重建的非零退出码，
/// 两项独立检查防止把部分输出误判为成功。
pub fn job_failure(
    solver_aborted: bool,
    exit_ok: bool,
    copy_back_error: Option<String>,
) -> Option<String> {
    if solver_aborted {
        Some("求解器报错退出（输出含 FOAM FATAL，详见作业日志）".to_string())
    } else if !exit_ok {
        Some("进程异常退出（输出无求解器错误标记）".to_string())
    } else {
        // 走到这里 = 求解正常结束；结果回传失败仍算失败，避免静默丢结果。
        copy_back_error
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn oversized_requests_are_rejected_before_entering_queue() {
        let mut jobs = Vec::new();
        assert!(super::submit(&mut jobs, "j".into(), None, "case".into(), 9, 0).is_err());
        assert!(jobs.is_empty());
        super::validate_cores(super::MAX_JOB_CORES).unwrap();
    }

    use super::*;

    fn seed() -> Vec<Job> {
        Vec::new()
    }

    #[test]
    fn submit_enters_queue() {
        let mut jobs = seed();
        submit(&mut jobs, "j1".into(), None, "/c".into(), 4, 100).unwrap();
        assert_eq!(jobs[0].status, JobStatus::Queued);
    }

    #[test]
    fn promote_respects_concurrent_and_cores_budget() {
        let limits = SchedulerLimits::new(2, 6);
        let mut jobs = seed();
        submit(&mut jobs, "a".into(), None, "/a".into(), 4, 0).unwrap();
        submit(&mut jobs, "b".into(), None, "/b".into(), 4, 0).unwrap();
        submit(&mut jobs, "c".into(), None, "/c".into(), 2, 0).unwrap();
        let started = promote_ready(&mut jobs, &limits, 10);
        // a(4) 先占 4 核；b 需 4 核会超 6 核预算 → 跳过；c 需 2 核可运行
        assert_eq!(started, vec!["a".to_string(), "c".to_string()]);
        assert_eq!(jobs[0].status, JobStatus::Running);
        assert_eq!(jobs[1].status, JobStatus::Queued);
        assert_eq!(jobs[2].status, JobStatus::Running);
    }

    #[test]
    fn cancel_from_queued_and_running() {
        let limits = SchedulerLimits::new(2, 8);
        let mut jobs = seed();
        submit(&mut jobs, "a".into(), None, "/a".into(), 2, 0).unwrap();
        submit(&mut jobs, "b".into(), None, "/b".into(), 2, 0).unwrap();
        promote_ready(&mut jobs, &limits, 1);
        cancel(&mut jobs, "b", 2).unwrap(); // 排队中取消
        cancel(&mut jobs, "a", 3).unwrap(); // 运行中取消
        assert_eq!(jobs[1].status, JobStatus::Cancelled);
        assert_eq!(jobs[0].status, JobStatus::Cancelled);
    }

    #[test]
    fn illegal_transitions_are_rejected() {
        let limits = SchedulerLimits::new(2, 8);
        let mut jobs = seed();
        submit(&mut jobs, "a".into(), None, "/a".into(), 2, 0).unwrap();
        assert!(mark_done(&mut jobs, "a", 1).is_err(), "排队不可直接完成");
        promote_ready(&mut jobs, &limits, 1);
        mark_done(&mut jobs, "a", 2).unwrap();
        assert!(cancel(&mut jobs, "a", 3).is_err(), "完成态不可取消");
        assert!(
            update_progress(&mut jobs, "a", 1.0).is_err(),
            "完成态不可更新进度"
        );
    }

    #[test]
    fn failed_carries_message() {
        let limits = SchedulerLimits::new(1, 8);
        let mut jobs = seed();
        submit(&mut jobs, "a".into(), None, "/a".into(), 2, 0).unwrap();
        promote_ready(&mut jobs, &limits, 1);
        mark_failed(&mut jobs, "a", "求解发散", 2).unwrap();
        assert_eq!(jobs[0].status, JobStatus::Failed);
        assert_eq!(jobs[0].message.as_deref(), Some("求解发散"));
    }

    #[test]
    fn job_failure_prefers_solver_abort_over_exit_code() {
        // 求解器错误标记独立于退出状态；即便外部进程返回 0，也不能忽略 FATAL。
        let aborted = job_failure(true, true, None).unwrap();
        assert!(aborted.contains("求解器报错退出"), "{aborted}");
        assert!(job_failure(true, false, None).is_some());
        // 无错误标记的非零退出 = 进程被终止 / 崩溃
        let crashed = job_failure(false, false, None).unwrap();
        assert!(crashed.contains("进程异常退出"), "{crashed}");
        // 正常结束：回传失败算失败，回传成功即成功
        let copy_error = job_failure(false, true, Some("回传失败".into())).unwrap();
        assert_eq!(copy_error, "回传失败");
        assert!(job_failure(false, true, None).is_none());
    }
}
