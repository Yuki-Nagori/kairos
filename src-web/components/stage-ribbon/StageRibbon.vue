<script setup lang="ts">
/** 阶段功能工具条：随分析阶段切换命令分组（逻辑见 useStageRibbon），样式对齐设计稿扁平工具条。 */
import { useStageRibbon } from "./useStageRibbon";

const { groups, hint } = useStageRibbon();
</script>

<template>
  <div
    class="flex h-14 shrink-0 items-center gap-1 overflow-x-auto border-b border-zinc-800 bg-zinc-900/60 px-2.5"
  >
    <template v-if="groups.length > 0">
      <div
        v-for="(group, index) in groups"
        :key="index"
        class="flex items-center gap-0.5 px-2"
        :class="{ 'border-r border-zinc-800': index < groups.length - 1 }"
      >
        <button
          v-for="button in group.buttons"
          :key="button.id"
          type="button"
          class="flex min-w-14 flex-col items-center gap-0.5 rounded-md px-2.5 py-1 transition-colors enabled:hover:bg-zinc-800/70 disabled:opacity-35"
          :disabled="button.disabled?.() ?? false"
          @click="button.run()"
        >
          <span
            class="grid size-5 place-items-center rounded bg-zinc-800 text-[11px] text-emerald-400"
            >{{ button.icon }}</span
          >
          <span class="whitespace-nowrap text-[10px] leading-3 text-zinc-400">{{
            button.label
          }}</span>
        </button>
      </div>
    </template>
    <p v-else class="px-2 text-xs text-zinc-600">{{ hint }}</p>
  </div>
</template>
