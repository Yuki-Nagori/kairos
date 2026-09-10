<script setup lang="ts">
/**
 * 分析阶段选项卡（v2 设计稿）：主页/几何/网格/工艺/求解/结果/报告。
 * 读写 store.stage，各列面板按阶段显隐由布局层（Phase 4 的 App 编排）负责。
 */
import { useStageTabs } from "./useStageTabs";

const { STAGES, app, active } = useStageTabs();
</script>

<template>
  <div class="flex items-center gap-1">
    <button
      v-for="[stage, label] in STAGES"
      :key="stage"
      type="button"
      class="border-b-2 border-transparent px-4 text-xs text-zinc-500 transition-colors hover:text-zinc-200"
      :class="{
        'border-emerald-400 font-semibold text-emerald-400': stage === active,
      }"
      @click="app.stage = stage"
    >
      {{ label }}
    </button>
  </div>
</template>
