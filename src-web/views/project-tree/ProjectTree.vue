<script setup lang="ts">
/** 项目树面板：逻辑见 useProjectTree。头部为「工程 + 当前项目徽标」。 */
import { useProjectTree } from "./useProjectTree";

const { groups, projectName } = useProjectTree();
</script>

<template>
  <aside
    class="project-tree min-w-0 shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900"
  >
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none"
    >
      <h2>工程</h2>
      <span
        class="ml-auto max-w-36 truncate rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-normal text-zinc-400"
        >{{ projectName }}</span
      >
    </div>
    <div class="px-2 py-2">
      <template v-for="group in groups" :key="group.title">
        <div
          class="flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-semibold text-zinc-500"
        >
          <span class="w-2.5 text-center text-[9px] text-zinc-600">▾</span>
          {{ group.title }}
        </div>
        <div
          v-for="leaf in group.leaves"
          :key="leaf"
          class="ml-4 flex items-center gap-1.5 rounded-md py-1 pl-2 pr-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800/50"
        >
          <span class="size-1.5 shrink-0 rounded-sm bg-lime-700" />
          <span class="truncate">{{ leaf }}</span>
        </div>
      </template>
    </div>
  </aside>
</template>
