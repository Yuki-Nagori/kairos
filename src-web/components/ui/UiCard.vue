<script setup lang="ts">
/**
 * 通用卡片：标题/图标插槽/状态提示行/刷新按钮/折叠（折叠偏好存 localStorage）。
 * 「探测 → 反馈」模式：statusHint 显示状态，提供 refreshLabel 则在正文顶部渲染刷新按钮。
 * 可折叠卡片的标题栏按按钮暴露（role=button + 回车/空格），键盘与读屏可达。
 */
import { ref } from "vue";
import UiButton from "./UiButton.vue";
import { storageGet, storageKey, storageSet } from "../../utils/storage";

const props = withDefaults(
  defineProps<{
    title: string;
    collapsible?: boolean;
    statusHint?: string;
    /** 提供则在正文顶部渲染刷新按钮（点击发 refresh 事件）。 */
    refreshLabel?: string;
  }>(),
  { collapsible: false, statusHint: "", refreshLabel: "" },
);

defineEmits<{ refresh: [] }>();

const collapsed = ref(
  props.collapsible ? storageGet(storageKey("panel", props.title), false) : false,
);

function toggleCollapse(): void {
  if (!props.collapsible) {
    return;
  }
  collapsed.value = !collapsed.value;
  storageSet(storageKey("panel", props.title), collapsed.value);
}
</script>

<template>
  <section class="min-w-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900">
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none"
      :class="{ 'cursor-pointer': collapsible }"
      :role="collapsible ? 'button' : undefined"
      :tabindex="collapsible ? 0 : undefined"
      :aria-expanded="collapsible ? !collapsed : undefined"
      @click="toggleCollapse"
      @keydown.enter.prevent="toggleCollapse"
      @keydown.space.prevent="toggleCollapse"
    >
      <slot name="icon" />
      <h2>{{ title }}</h2>
      <span v-if="collapsible" class="ml-auto text-zinc-500">{{ collapsed ? "▸" : "▾" }}</span>
    </div>
    <div class="min-w-0 space-y-3 overflow-x-hidden px-4 py-3.5" :class="{ hidden: collapsed }">
      <UiButton v-if="refreshLabel" @click="$emit('refresh')">{{ refreshLabel }}</UiButton>
      <p v-if="statusHint" class="text-xs text-zinc-500">{{ statusHint }}</p>
      <slot />
    </div>
  </section>
</template>
