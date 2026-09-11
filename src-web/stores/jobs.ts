/** 求解作业状态：调度器持有的作业列表、每作业日志尾部（环形缓冲）与环境探测。 */
import { Channel } from "@tauri-apps/api/core";
import { defineStore } from "pinia";
import { cancelJob as apiCancelJob, listJobs, submitJob as apiSubmitJob } from "../api/jobs";
import { probeOpenfoam as apiProbeOpenfoam } from "../api/solver";
import type { EnvironmentCheck, Job } from "../types";
import { useAppStore } from "./app";
import { useProjectStore } from "./project";

/** 每作业日志的环形上限：超出后丢弃最旧行，避免长作业撑爆内存。 */
const JOB_LOG_LIMIT = 200;

export const useJobsStore = defineStore("jobs", {
  state: () => ({
    /** 求解作业列表（调度器持有的快照）。 */
    jobs: [] as Job[],
    /** 每作业的求解日志尾部（环形缓冲，key = 作业 id）。 */
    jobLogs: {} as Record<string, string[]>,
    /** OpenFOAM 环境探测结果（null = 尚未完成）。 */
    envCheck: null as EnvironmentCheck | null,
  }),
  actions: {
    /** 探测 OpenFOAM 环境（作业面板顶部环境行）；失败进全局错误。 */
    async probeOpenfoam(): Promise<void> {
      const app = useAppStore();
      try {
        this.envCheck = await apiProbeOpenfoam();
      } catch (error) {
        app.setError(error);
      }
    },
    /** 追加一行作业日志（环形缓冲）。 */
    appendJobLog(jobId: string, line: string): void {
      const logs = this.jobLogs[jobId] ?? [];
      const next = [...logs, line];
      if (next.length > JOB_LOG_LIMIT) {
        next.splice(0, next.length - JOB_LOG_LIMIT);
      }
      this.jobLogs = { ...this.jobLogs, [jobId]: next };
    },
    /** 提交求解作业（case 目录 + 核数）。Rust 侧解析时间标记写入作业进度并经
     * Channel 转发原始日志行；日志按作业缓存进 store 供作业面板展示。 */
    async submitJob(caseDir: string, cores: number): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在提交作业…", async () => {
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
          this.appendJobLog(jobId, line);
        };
        const job = await apiSubmitJob(caseDir, cores, useProjectStore().activeStudyId, channel);
        jobId = job.id;
        const buffered = pending.splice(0);
        if (buffered.length > 0) {
          buffered.forEach((line) => this.appendJobLog(jobId as string, line));
        }
        await this.refreshJobs();
      });
    },
    async cancelJob(jobId: string): Promise<void> {
      const app = useAppStore();
      try {
        await apiCancelJob(jobId);
        await this.refreshJobs();
      } catch (error) {
        app.setError(error);
      }
    },
    async refreshJobs(): Promise<void> {
      const app = useAppStore();
      try {
        this.jobs = await listJobs();
      } catch (error) {
        app.setError(error);
      }
    },
  },
});
