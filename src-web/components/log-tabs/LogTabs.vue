<script setup lang="ts">
/** 日志标签组：逻辑见 useLogTabs。 */
import { useLogTabs } from "./useLogTabs";

const { TABS, active, lines } = useLogTabs();
</script>

<template>
  <div class="mt-2 flex h-44 flex-col overflow-hidden rounded-lg border border-zinc-800 bg-black">
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[10px] text-zinc-400"
    >
      <button
        v-for="tab in TABS"
        :key="tab"
        type="button"
        class="rounded px-2 py-0.5"
        :class="
          tab === active ? 'bg-zinc-800 text-emerald-400' : 'text-zinc-500 hover:text-zinc-200'
        "
        @click="active = tab"
      >
        {{ tab }}
      </button>
    </div>
    <pre
      class="flex-1 overflow-y-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-4 text-emerald-300/90"
      >{{ lines.length > 0 ? lines.join("\n") : "（暂无日志）" }}</pre>
  </div>
</template>
