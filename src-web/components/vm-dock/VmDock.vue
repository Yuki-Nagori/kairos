<script setup lang="ts">
/**
 * 虚拟机终端抽屉：浮于工作区右下角（不占布局列），显隐由
 * 状态栏右侧 Shell 按钮与原生菜单驱动（store.vmPanelVisible）。
 * 面板本体经默认插槽注入（Phase 4 换成 VmPanel.vue）。
 * 纯 v-show 包装无逻辑，故不建 composable。
 */
import { computed } from "vue";
import { useVmStore } from "../../stores/vm";

const vm = useVmStore();
const visible = computed(() => vm.vmPanelVisible);
</script>

<template>
  <div v-show="visible" class="absolute right-2 bottom-2 z-40 w-104">
    <slot />
  </div>
</template>
