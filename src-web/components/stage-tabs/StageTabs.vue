<script setup lang="ts">
/** 分析阶段选项卡：逻辑见 useStageTabs。样式为设计稿的独立选项卡行（下划线高亮），
 *  行内角标提示方案任务中需要立即注意的状态（几何告警 / 求解执行中 / 失败）。 */
import { useStageTabs } from "./useStageTabs";

const { STAGES, app, active, badgeOf } = useStageTabs();
</script>

<template>
  <div class="flex h-full items-stretch gap-1">
    <button
      v-for="[stage, label] in STAGES"
      :key="stage"
      type="button"
      class="flex items-center gap-1 border-b-2 px-4 text-xs transition-colors"
      :class="
        stage === active
          ? 'border-emerald-400 font-semibold text-emerald-400'
          : 'border-transparent text-zinc-500 hover:text-zinc-200'
      "
      @click="app.stage = stage"
    >
      {{ label }}
      <span
        v-if="badgeOf(stage)"
        class="text-[10px] font-semibold"
        :class="badgeOf(stage)!.cls"
        :title="badgeOf(stage)!.title"
        >{{ badgeOf(stage)!.icon }}</span
      >
    </button>
  </div>
</template>
