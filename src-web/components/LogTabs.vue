<script setup lang="ts">
/**
 * 中列底部日志标签组（v2 设计稿）：分析日志 / 网格日志 / VM 终端。
 * 只读镜像——交互式输入仍在 VM 抽屉；数据全部来自 store 分片。
 */
import { computed, ref } from "vue";
import { useJobsStore } from "../stores/jobs";
import { useGeometryStore } from "../stores/geometry";
import { useVmStore } from "../stores/vm";

const TABS = ["分析日志", "网格日志", "VM 终端"] as const;
type Tab = (typeof TABS)[number];

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
</script>

<template>
  <div class="mt-2 flex h-44 flex-col overflow-hidden rounded-lg border border-zinc-800 bg-black">
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[10px] text-zinc-400"
    >
      <button
        v-for="tab in TABS"
        :key="tab"
        type="button"
        class="rounded px-2 py-0.5"
        :class="
          tab === active ? 'bg-zinc-800 text-emerald-400' : 'text-zinc-500 hover:text-zinc-200'
        "
        @click="active = tab"
      >
        {{ tab }}
      </button>
    </div>
    <pre
      class="flex-1 overflow-y-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-4 text-emerald-300/90"
      >{{ lines.length > 0 ? lines.join("\n") : "（暂无日志）" }}</pre>
  </div>
</template>
