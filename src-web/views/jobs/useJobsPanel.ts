/** 求解作业面板：列表、提交与取消（并发预算由调度器控制）；顶部在求解
 *  环境「已下载新版本但 VM 未部署」时展示提醒横幅（T54）。 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useJobsStore } from "../../stores/jobs";
import { useDependenciesStore } from "../../stores/dependencies";
import { useVmStore } from "../../stores/vm";
import { isPendingDeploy } from "../../utils/deploy";
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
  const deps = useDependenciesStore();
  const vm = useVmStore();

  /** 求解环境「已下载新版本但 VM 内未部署」提醒（T54）。 */
  const pendingDeployText = computed(() => {
    const downloaded = deps.downloadedFiles["moldingfoam"]?.releaseTag ?? null;
    if (downloaded === null || !isPendingDeploy(downloaded, vm.deployedReleaseTag)) {
      return null;
    }
    return `求解环境有更新未部署（${downloaded}）：提交的作业将使用 VM 内旧环境，请先到依赖面板「部署到虚拟机」`;
  });

  const caseDir = ref("");
  const cores = ref("");

  function submitJob(): void {
    const dir = caseDir.value.trim();
    if (!dir) {
      return;
    }
    void jobsStore.submitJob(dir, Number(cores.value) || 2);
  }

  // 环境探测行：探测经 jobs store（错误进全局管道），完成后整体替换样式。
  void jobsStore.probeOpenfoam();
  const envHint = computed(() => jobsStore.envCheck?.hint ?? "正在探测 OpenFOAM 环境…");
  const envClass = computed(() => {
    const check = jobsStore.envCheck;
    if (!check) {
      return "text-xs text-zinc-500";
    }
    return check.openfoam && check.solver ? "text-emerald-400" : "text-amber-400";
  });

  function jobLogsTail(jobId: string): string[] {
    return jobsStore.jobLogs[jobId] ?? [];
  }

  return {
    app,
    jobsStore,
    pendingDeployText,
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
