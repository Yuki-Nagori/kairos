/** 求解作业面板：列表、提交与取消（并发预算由调度器控制）。 */
import { ref } from "vue";
import { probeOpenfoam } from "../../api/solver";
import { useAppStore } from "../../stores/app";
import { useJobsStore } from "../../stores/jobs";
import type { Job } from "../../types";

export function useJobsPanel() {
  const STATUS_LABEL: Record<Job["status"], string> = {
    queued: "排队中",
    running: "运行中",
    done: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };

  const STATUS_CLASS: Record<Job["status"], string> = {
    queued: "text-zinc-400",
    running: "text-amber-300",
    done: "text-emerald-400",
    failed: "text-red-400",
    cancelled: "text-zinc-500",
  };

  const app = useAppStore();
  const jobsStore = useJobsStore();

  const caseDir = ref("");
  const cores = ref("");

  function submitJob(): void {
    const dir = caseDir.value.trim();
    if (!dir) {
      return;
    }
    void jobsStore.submitJob(dir, Number(cores.value) || 2);
  }

  // 环境探测行：探测完成后 className 整体替换（不再带 text-xs），是有意行为。
  const envHint = ref("正在探测 OpenFOAM 环境…");
  const envClass = ref("text-xs text-zinc-500");
  void probeOpenfoam().then((check) => {
    envHint.value = check.hint;
    envClass.value = check.openfoam && check.solver ? "text-emerald-400" : "text-amber-400";
  });

  function jobLogsTail(jobId: string): string[] {
    return jobsStore.jobLogs[jobId] ?? [];
  }

  return {
    app,
    jobsStore,
    caseDir,
    cores,
    submitJob,
    envHint,
    envClass,
    STATUS_LABEL,
    STATUS_CLASS,
    jobLogsTail,
  };
}
