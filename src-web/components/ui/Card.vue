/** * 通用卡片 Vue 组件：标题/图标/状态行/刷新按钮/折叠。 */
<script setup lang="ts">
import { ref } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    collapsible?: boolean;
    statusHint?: string;
  }>(),
  { collapsible: false, statusHint: "" },
);

const emit = defineEmits<{ refresh: [] }>();

const collapsed = ref(
  props.collapsible ? localStorage.getItem(`kairos-panel:${props.title}`) === "1" : false,
);

function toggleCollapse(): void {
  collapsed.value = !collapsed.value;
  localStorage.setItem(`kairos-panel:${props.title}`, collapsed.value ? "1" : "0");
}
</script>

<template>
  <section class="min-w-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900">
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none"
      :class="{ 'cursor-pointer': collapsible }"
      @click="collapsible && toggleCollapse()"
    >
      <slot name="icon" />
      <span>{{ title }}</span>
      <span v-if="collapsible" class="ml-auto text-zinc-500" style="font-size: 14px">
        {{ collapsed ? "▸" : "▾" }}
      </span>
    </div>
    <div class="min-w-0 space-y-3 overflow-x-hidden px-4 py-3.5" :class="{ hidden: collapsed }">
      <p v-if="statusHint" class="text-xs text-zinc-500">{{ statusHint }}</p>
      <slot />
    </div>
  </section>
</template>
