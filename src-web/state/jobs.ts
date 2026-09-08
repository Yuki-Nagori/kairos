import { Channel } from "@tauri-apps/api/core";
import { cancelJob as apiCancelJob, listJobs, submitJob as apiSubmitJob } from "../services/jobs";
import { appStore, setError } from "./store";

/** 每作业日志的环形上限：超出后丢弃最旧行，避免长作业撑爆内存。 */
const JOB_LOG_LIMIT = 200;

/** 提交求解作业（case 目录 + 核数）。Rust 侧解析时间标记写入作业进度并经
 * Channel 转发原始日志行；日志按作业缓存进 store 供作业面板展示。 */
export async function submitJobAction(caseDir: string, cores: number): Promise<void> {
  appStore.set({ busy: "正在提交作业…", error: null });
  const channel = new Channel<string>();
  // invoke 尚未返回时日志就可能到达：先攒进 pending，拿到 jobId 后一次性入账。
  const pending: string[] = [];
  let jobId: string | null = null;
  channel.onmessage = (line) => {
    if (line.startsWith("__TIME__")) {
      return; // 时间标记由调度器解析，不经前端。
    }
    if (jobId === null) {
      pending.push(line);
      return;
    }
    appendJobLog(jobId, line);
  };
  try {
    const job = await apiSubmitJob(caseDir, cores, appStore.get().activeStudyId, channel);
    jobId = job.id;
    const buffered = pending.splice(0);
    if (buffered.length > 0) {
      buffered.forEach((line) => appendJobLog(jobId as string, line));
    }
    await refreshJobs();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

function appendJobLog(jobId: string, line: string): void {
  const logs = appStore.get().jobLogs[jobId] ?? [];
  const next = [...logs, line];
  if (next.length > JOB_LOG_LIMIT) {
    next.splice(0, next.length - JOB_LOG_LIMIT);
  }
  appStore.set({ jobLogs: { ...appStore.get().jobLogs, [jobId]: next } });
}

export async function cancelJobAction(jobId: string): Promise<void> {
  try {
    await apiCancelJob(jobId);
    await refreshJobs();
  } catch (error) {
    setError(error);
  }
}

export async function refreshJobs(): Promise<void> {
  try {
    const jobs = await listJobs();
    appStore.set({ jobs });
  } catch (error) {
    setError(error);
  }
}
