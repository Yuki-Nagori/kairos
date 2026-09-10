<script setup lang="ts">
/** 层管理面板：图层显隐控制（◉ 开 / ◎ 熄），样式对齐设计稿层列表。 */
import { useLayersPanel } from "./useLayersPanel";

const { rows, toggle } = useLayersPanel();
</script>

<template>
  <section class="min-w-0 shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900">
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none"
    >
      <h2>层</h2>
      <span class="ml-auto text-[10px] font-normal text-zinc-500">{{ rows.length }}</span>
    </div>
    <div class="space-y-0.5 px-2 py-2">
      <button
        v-for="row in rows"
        :key="row.id"
        type="button"
        class="flex w-full items-center gap-2 rounded-md px-1.5 py-1 text-left transition-colors"
        :class="row.available ? 'hover:bg-zinc-800/60' : 'cursor-default opacity-45'"
        :title="row.available ? `切换${row.label}图层` : `尚无${row.label}数据`"
        @click="toggle(row)"
      >
        <span
          class="w-4 text-center text-xs"
          :class="row.available && row.visible ? 'text-emerald-400' : 'text-zinc-600'"
          >{{ row.available && row.visible ? "◉" : "◎" }}</span
        >
        <span class="text-xs" :class="row.available ? 'text-zinc-300' : 'text-zinc-500'">{{
          row.label
        }}</span>
        <span v-if="row.count !== null" class="ml-auto font-mono text-[10px] text-zinc-500">{{
          row.count
        }}</span>
      </button>
    </div>
  </section>
</template>
