<script setup lang="ts">
/** 底部状态栏：版本/IPC、忙碌、错误三态（优先级：错误 > 忙碌 > 版本）。 */
import { computed } from "vue";
import { toggleVmPanel, useAppState } from "../state";
import ShellIcon from "./ui/ShellIcon.vue";

const state = useAppState();

const status = computed(() => {
  if (state.error) {
    return {
      text: state.error.message,
      class: state.error.info ? "text-zinc-400" : "text-red-400",
    };
  }
  if (state.busy) {
    return { text: state.busy, class: "text-amber-300" };
  }
  if (state.info) {
    return {
      text: `v${state.info.version} · ${state.info.os} · IPC 正常`,
      class: "text-zinc-500",
    };
  }
  return { text: "", class: "" };
});

// Shell 环境入口：点击切换右下角虚拟机终端面板的显隐。
const shellVisible = computed(() => state.vmPanelVisible);
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
      @click="toggleVmPanel()"
    >
      <span>{{ shellVisible ? "Shell 环境（点击收起）" : "Shell 环境" }}</span>
      <ShellIcon class="h-4 w-4" />
    </button>
  </footer>
</template>
