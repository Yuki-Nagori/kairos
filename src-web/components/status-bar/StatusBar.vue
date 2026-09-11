<script setup lang="ts">
/** 底部状态栏：左侧三态状态 · 中部求解进度 · 右侧版本与 Shell 入口（逻辑见 useStatusBar）。 */
import ShellIcon from "../ui/icons/ShellIcon.vue";
import { useStatusBar } from "./useStatusBar";

const { vm, status, solveProgress, versionText, shellVisible } = useStatusBar();
</script>

<template>
  <footer
    class="flex shrink-0 items-center gap-4 border-t border-zinc-800 bg-zinc-900 px-4 py-1.5 select-none"
  >
    <span class="text-[11px]" :class="status.class">
      <span
        v-if="status.code"
        class="mr-1 rounded bg-zinc-800 px-1 font-mono text-[10px] text-zinc-400"
        >{{ status.code }}</span
      >{{ status.text }}</span
    >
    <span v-if="status.hint" class="text-[11px] text-zinc-500">{{ status.hint }}</span>
    <span v-if="solveProgress" class="text-[11px] tabular-nums text-zinc-400">{{
      solveProgress
    }}</span>
    <span class="flex-1" />
    <span v-if="versionText" class="text-[11px] text-zinc-500">{{ versionText }}</span>
    <button
      type="button"
      class="flex items-center gap-1.5 text-[11px] transition-colors"
      :class="shellVisible ? 'text-emerald-400' : 'text-zinc-400 hover:text-zinc-200'"
      @click="vm.togglePanel()"
    >
      <span>{{ shellVisible ? "Shell 环境（点击收起）" : "Shell 环境" }}</span>
      <ShellIcon class="h-4 w-4" />
    </button>
  </footer>
</template>
