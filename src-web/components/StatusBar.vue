<script setup lang="ts">
/** 底部状态栏：版本/IPC、忙碌、错误三态（优先级：错误 > 忙碌 > 版本）。 */
import { computed } from "vue";
import { useAppStore } from "../stores/app";
import { useVmStore } from "../stores/vm";
import ShellIcon from "./ui/ShellIcon.vue";

const app = useAppStore();
const vm = useVmStore();

const status = computed(() => {
  if (app.error) {
    return {
      text: app.error.message,
      class: app.error.info ? "text-zinc-400" : "text-red-400",
    };
  }
  if (app.busy) {
    return { text: app.busy, class: "text-amber-300" };
  }
  if (app.info) {
    return {
      text: `v${app.info.version} · ${app.info.os} · IPC 正常`,
      class: "text-zinc-500",
    };
  }
  return { text: "", class: "" };
});

// Shell 环境入口：点击切换右下角虚拟机终端面板的显隐。
const shellVisible = computed(() => vm.vmPanelVisible);
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
