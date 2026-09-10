<script setup lang="ts">
/** 项目树面板：层级展示工程 / 几何 / 求解作业 / 运行时依赖。 */
import { computed } from "vue";
import { useAppState } from "../../state";

const state = useAppState();

// 空分组不渲染，保持树的紧凑；项目组恒在（未打开时给占位叶）。
const groups = computed(() => {
  const sections: { title: string; leaves: string[] }[] = [
    { title: "项目", leaves: [state.project?.name ?? "未打开项目"] },
    {
      title: "几何",
      leaves: state.geometries.map((geo) => `${geo.fileName} (${geo.triangleCount} 面)`),
    },
    { title: "求解作业", leaves: state.jobs.map((job) => `${job.id}: ${job.status}`) },
    {
      title: "运行时依赖",
      leaves: state.dependencies.map((dep) => `${dep.name}: ${dep.ready ? "就绪" : "未就绪"}`),
    },
  ];
  return sections.filter((section) => section.leaves.length > 0);
});
</script>

<template>
  <aside class="project-tree">
    <div class="flex items-center gap-2 rounded-md px-2 py-1 text-xs font-semibold text-zinc-200">
      工程浏览器
    </div>
    <div class="tree-body">
      <template v-for="group in groups" :key="group.title">
        <div
          class="flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-semibold text-zinc-400"
        >
          {{ group.title }}
        </div>
        <div
          v-for="leaf in group.leaves"
          :key="leaf"
          class="ml-3 flex items-center gap-1.5 rounded-md border-l border-zinc-800 py-1 pl-3 pr-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800/50"
        >
          {{ leaf }}
        </div>
      </template>
    </div>
  </aside>
</template>
