import { invokeCommand } from "../lib/ipc";
import type { Channel } from "@tauri-apps/api/core";
import type { Job } from "../types";

/** 提交求解作业（入队，按预算自动启动）。progress 通道回传求解日志行，前端暂未消费。 */
export function submitJob(
  caseDir: string,
  cores: number,
  studyId: string | null,
  progress: Channel<string>,
): Promise<Job> {
  return invokeCommand("submit_job", { caseDir, cores, studyId, progress });
}

export function cancelJob(jobId: string): Promise<void> {
  return invokeCommand("cancel_job", { jobId });
}

export function listJobs(): Promise<Job[]> {
  return invokeCommand("list_jobs");
}
