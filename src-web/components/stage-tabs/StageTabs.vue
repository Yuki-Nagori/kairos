<script setup lang="ts">
/** 分析阶段选项卡：逻辑见 useStageTabs。样式为设计稿的独立选项卡行（下划线高亮）。 */
import { useStageTabs } from "./useStageTabs";

const { STAGES, app, active } = useStageTabs();
</script>

<template>
  <div class="flex h-full items-stretch gap-1">
    <button
      v-for="[stage, label] in STAGES"
      :key="stage"
      type="button"
      class="flex items-center border-b-2 px-4 text-xs transition-colors"
      :class="
        stage === active
          ? 'border-emerald-400 font-semibold text-emerald-400'
          : 'border-transparent text-zinc-500 hover:text-zinc-200'
      "
      @click="app.stage = stage"
    >
      {{ label }}
    </button>
  </div>
</template>
