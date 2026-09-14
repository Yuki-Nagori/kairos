/** 求解作业面板：列表、提交与取消（并发预算由调度器控制）；顶部在求解
 *  环境「已下载新版本但 VM 未部署」时展示提醒横幅。 */
import { computed, onScopeDispose, ref, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useJobsStore } from "../../stores/jobs";
import { useDependenciesStore } from "../../stores/dependencies";
import { useVmStore } from "../../stores/vm";
import { isPendingDeploy } from "../../utils/deploy";
import type { Job } from "../../types";

/** 作业运行期间的列表轮询间隔（毫秒）：作业列表是后端快照，不轮询就会一直
 *  停在提交时的状态（作业早已失败，界面还显示「运行中」）。 */
const JOB_POLL_MS = 1500;

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

  /** 求解环境「已下载新版本但未部署」提醒（VM 通道 / Linux 原生通道措辞不同）。 */
  const pendingDeployText = computed(() => {
    const downloaded = deps.downloadedFiles["moldingfoam"]?.releaseTag ?? null;
    if (downloaded === null || !isPendingDeploy(downloaded, vm.deployedReleaseTag)) {
      return null;
    }
    if (vm.vmStatus?.provider === "native") {
      return `求解环境有更新未部署（${downloaded}）：提交的作业将使用本机旧环境，请先到依赖面板「部署到本机」`;
    }
    return `求解环境有更新未部署（${downloaded}）：提交的作业将使用 VM 内旧环境，请先到依赖面板「部署到虚拟机」`;
  });

  const caseDir = ref("");
  const cores = ref("");

  /** 是否存在未收尾的作业（排队 / 运行中）——决定是否轮询列表。 */
  const hasActiveJob = computed(() =>
    jobsStore.jobs.some((job) => job.status === "running" || job.status === "queued"),
  );

  // 作业在跑时定期刷新列表：作业结束（成功或失败）后自动停表，面板卸载时清理。
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  function stopPolling(): void {
    if (pollTimer !== null) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }
  watch(
    hasActiveJob,
    (active) => {
      stopPolling();
      if (active) {
        pollTimer = setInterval(() => {
          void jobsStore.refreshJobs();
        }, JOB_POLL_MS);
      }
    },
    { immediate: true },
  );
  onScopeDispose(stopPolling);

  function submitJob(): void {
    const dir = caseDir.value.trim();
    if (!dir) {
      return;
    }
    void jobsStore.submitJob(dir, Number(cores.value) || 2);
  }

  // 环境探测行：探测经 jobs store（错误进全局管道），完成后整体替换样式。
  void jobsStore.probeMoldingfoam();
  const envHint = computed(() => jobsStore.envCheck?.hint ?? "正在探测求解环境…");
  const envClass = computed(() => {
    const check = jobsStore.envCheck;
    if (!check) {
      return "text-xs text-zinc-500";
    }
    return check.moldingfoam && check.solver ? "text-emerald-400" : "text-amber-400";
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
