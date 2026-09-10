import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { Channel } from "@tauri-apps/api/core";
import { useAppStore } from "../../../src-web/stores/app";
import { useJobsStore } from "../../../src-web/stores/jobs";
import { useProjectStore } from "../../../src-web/stores/project";
import { cancelJob, listJobs, submitJob } from "../../../src-web/api/jobs";
import type { Job } from "../../../src-web/types";

// happy-dom 下没有 Tauri IPC：用可赋值 onmessage 的桩替换 Channel。
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((message: unknown) => void) | null = null;
  },
}));
vi.mock("../../../src-web/api/jobs", () => ({
  submitJob: vi.fn(),
  cancelJob: vi.fn(),
  listJobs: vi.fn(),
}));

function makeJob(id = "job-1"): Job {
  return {
    id,
    studyId: "s-1",
    caseDir: "/case",
    cores: 4,
    status: "queued",
    createdMs: 1,
    startedMs: null,
    finishedMs: null,
    lastTimeS: null,
    message: null,
  };
}

describe("jobs store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetAllMocks();
  });

  describe("appendJobLog", () => {
    it("appends log lines per job without touching other jobs", () => {
      const jobs = useJobsStore();
      jobs.appendJobLog("job-1", "第一行");
      jobs.appendJobLog("job-2", "另一作业");
      jobs.appendJobLog("job-1", "第二行");

      expect(jobs.jobLogs["job-1"]).toEqual(["第一行", "第二行"]);
      expect(jobs.jobLogs["job-2"]).toEqual(["另一作业"]);
    });

    it("caps each job log at 200 lines", () => {
      const jobs = useJobsStore();
      for (let index = 0; index < 205; index += 1) {
        jobs.appendJobLog("job-1", `line-${index}`);
      }

      expect(jobs.jobLogs["job-1"]).toHaveLength(200);
      expect(jobs.jobLogs["job-1"]?.[0]).toBe("line-5");
      expect(jobs.jobLogs["job-1"]?.at(-1)).toBe("line-204");
    });
  });

  describe("submitJob", () => {
    it("submits with the active study id and buffers early log lines", async () => {
      const project = useProjectStore();
      project.activeStudyId = "s-1";

      const channels: Channel<string>[] = [];
      vi.mocked(listJobs).mockResolvedValue([makeJob()]);

      const app = useAppStore();
      const jobs = useJobsStore();
      const busyDuring: (string | null)[] = [];
      vi.mocked(submitJob).mockImplementation(async (_caseDir, _cores, _studyId, progress) => {
        busyDuring.push(useAppStore().busy);
        channels.push(progress);
        // invoke 尚未返回：时间标记被丢弃，普通行先攒进 pending。
        progress.onmessage("__TIME__ 0.5");
        progress.onmessage("早到的日志");
        return makeJob();
      });

      await jobs.submitJob("/case", 4);

      // invoke 返回后到达的行直接入账，时间标记仍被忽略。
      channels[0]?.onmessage("__TIME__ 1.0");
      channels[0]?.onmessage("求解中");

      expect(submitJob).toHaveBeenCalledWith("/case", 4, "s-1", expect.anything());
      expect(jobs.jobLogs["job-1"]).toEqual(["早到的日志", "求解中"]);
      expect(jobs.jobs).toHaveLength(1);
      expect(busyDuring).toEqual(["正在提交作业…"]);
      expect(app.busy).toBeNull();
      expect(app.error).toBeNull();
    });

    it("appends directly when no lines arrive before the invoke resolves", async () => {
      vi.mocked(submitJob).mockResolvedValue(makeJob("job-2"));
      vi.mocked(listJobs).mockResolvedValue([]);

      const jobs = useJobsStore();
      await jobs.submitJob("/case", 2);

      expect(jobs.jobLogs["job-2"]).toBeUndefined();
      expect(jobs.jobs).toEqual([]);
    });

    it("reports submit failures and clears busy", async () => {
      vi.mocked(submitJob).mockRejectedValue(new Error("入队失败"));

      const app = useAppStore();
      const jobs = useJobsStore();
      await jobs.submitJob("/case", 4);

      expect(app.error?.message).toBe("入队失败");
      expect(app.busy).toBeNull();
      expect(jobs.jobs).toEqual([]);
    });
  });

  describe("cancelJob", () => {
    it("cancels and refreshes the job list", async () => {
      vi.mocked(cancelJob).mockResolvedValue(undefined);
      vi.mocked(listJobs).mockResolvedValue([makeJob("job-9")]);

      const app = useAppStore();
      const jobs = useJobsStore();
      await jobs.cancelJob("job-9");

      expect(cancelJob).toHaveBeenCalledWith("job-9");
      expect(jobs.jobs.map((job) => job.id)).toEqual(["job-9"]);
      expect(app.error).toBeNull();
    });

    it("reports cancel failures", async () => {
      vi.mocked(cancelJob).mockRejectedValue(new Error("取消失败"));

      const app = useAppStore();
      const jobs = useJobsStore();
      await jobs.cancelJob("job-9");

      expect(app.error?.message).toBe("取消失败");
    });
  });

  describe("refreshJobs", () => {
    it("stores the scheduler snapshot", async () => {
      vi.mocked(listJobs).mockResolvedValue([makeJob("job-1"), makeJob("job-2")]);

      const jobs = useJobsStore();
      await jobs.refreshJobs();

      expect(jobs.jobs).toHaveLength(2);
    });

    it("reports refresh failures", async () => {
      vi.mocked(listJobs).mockRejectedValue(new Error("调度器不可达"));

      const app = useAppStore();
      const jobs = useJobsStore();
      await jobs.refreshJobs();

      expect(app.error?.message).toBe("调度器不可达");
    });
  });
});
