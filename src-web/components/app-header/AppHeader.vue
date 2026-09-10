<script setup lang="ts">
/** 应用标题栏：品牌信息 + 阶段选项卡 + 主题切换（全局状态展示在底部状态栏）。 */
import StageTabs from "../stage-tabs/StageTabs.vue";
import { useAppHeader } from "./useAppHeader";

const { theme, cycleTheme } = useAppHeader();
</script>

<template>
  <header class="flex shrink-0 items-center gap-3 border-b border-zinc-800 bg-zinc-900 px-4 py-2">
    <img src="/icon.svg" alt="Kairos" class="h-6 w-6 shrink-0" />
    <h1 class="text-sm font-semibold tracking-wide">Kairos</h1>
    <span class="text-xs text-zinc-500">CAE 仿真</span>
    <span class="flex-1" />
    <!-- 分析阶段选项卡与品牌同行：logo | Kairos CAE 仿真 | [选项卡] | ... | 主题 -->
    <StageTabs />
    <button
      type="button"
      class="rounded-lg border border-zinc-700 px-2 py-1 text-xs text-zinc-300 transition-colors hover:border-emerald-500/70 hover:bg-emerald-500/10 hover:text-emerald-300"
      :title="theme === 'light' ? '切换到浅色主题' : '切换到深色主题'"
      @click="cycleTheme()"
    >
      {{ theme === "light" ? "☾" : "☀" }}
    </button>
  </header>
</template>
