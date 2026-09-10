<script setup lang="ts">
/**
 * 虚拟机终端抽屉：浮于工作区右下角（不占布局列），显隐由
 * 状态栏右侧 Shell 按钮与原生菜单驱动（store.vmPanelVisible）。
 * 面板本体经默认插槽注入（Phase 4 换成 VmPanel.vue）。
 */
import { onUnmounted, ref } from "vue";
import { select } from "../lib/store";
import { appStore } from "../state";

const visible = ref(appStore.get().vmPanelVisible);
const unsubscribe = select(
  appStore,
  (state) => state.vmPanelVisible,
  (value) => {
    visible.value = value;
  },
);
onUnmounted(unsubscribe);
</script>

<template>
  <div v-show="visible" class="absolute right-2 bottom-2 z-40 w-[26rem]">
    <slot />
  </div>
</template>
