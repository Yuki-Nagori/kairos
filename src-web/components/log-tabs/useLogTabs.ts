/**
 * 中列底部日志标签组逻辑：分析日志 / 网格日志 / VM 终端。
 * 只读镜像——交互式输入仍在 VM 抽屉；数据全部来自 store 分片。
 */
import { computed, ref } from "vue";
import { useJobsStore } from "../../stores/jobs";
import { useGeometryStore } from "../../stores/geometry";
import { useVmStore } from "../../stores/vm";

const TABS = ["分析日志", "网格日志", "VM 终端"] as const;
type Tab = (typeof TABS)[number];

export function useLogTabs() {
  const active = ref<Tab>(TABS[0]);
  const jobsStore = useJobsStore();
  const geometry = useGeometryStore();
  const vm = useVmStore();

  const lines = computed(() => {
    if (active.value === "分析日志") {
      const job = jobsStore.jobs.at(-1);
      return (job ? (jobsStore.jobLogs[job.id] ?? []) : []).slice(-40);
    }
    if (active.value === "网格日志") {
      const report = Object.values(geometry.meshReports).at(-1);
      return report
        ? [
            `引擎 ${report.engine} · ${report.nodeCount} 节点 / ${report.elementCount} 四面体`,
            `质量（长径比）min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`,
          ]
        : [];
    }
    return vm.vmShellLogs.slice(-40);
  });

  return { TABS, active, lines };
}
