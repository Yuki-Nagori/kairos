/** 求解作业 IPC：提交（日志经 Channel 回传）、取消与列表。 */
import { invokeCommand } from "../utils/ipc";
import type { Channel } from "@tauri-apps/api/core";
import type { Job } from "../types";

/** 提交求解作业：入队后按调度预算自动启动；求解日志行经 progress 通道实时回传。 */
export function submitJob(
  caseDir: string,
  cores: number,
  studyId: string | null,
  progress: Channel<string>,
): Promise<Job> {
  return invokeCommand("submit_job", { caseDir, cores, studyId, progress });
}

/** 取消排队 / 运行中的作业。 */
export function cancelJob(jobId: string): Promise<void> {
  return invokeCommand("cancel_job", { jobId });
}

/** 作业列表（调度器快照）。 */
export function listJobs(): Promise<Job[]> {
  return invokeCommand("list_jobs");
}
