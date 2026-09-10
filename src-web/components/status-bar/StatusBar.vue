<script setup lang="ts">
/** 底部状态栏：版本/IPC、忙碌、错误三态（优先级：错误 > 忙碌 > 版本）。 */
import ShellIcon from "../ui/icons/ShellIcon.vue";
import { useStatusBar } from "./useStatusBar";

const { vm, status, shellVisible } = useStatusBar();
</script>

<template>
  <footer
    class="flex shrink-0 items-center justify-between gap-4 border-t border-zinc-800 bg-zinc-900 px-4 py-1.5"
  >
    <span class="text-[11px]" :class="status.class">{{ status.text }}</span>
    <span class="flex-1" />
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
