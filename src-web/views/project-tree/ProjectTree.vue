<script setup lang="ts">
/** 项目树面板：逻辑见 useProjectTree。 */
import { useProjectTree } from "./useProjectTree";

const { groups } = useProjectTree();
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
