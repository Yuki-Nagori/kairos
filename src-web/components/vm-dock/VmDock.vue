<script setup lang="ts">
/**
 * 虚拟机终端抽屉：浮于工作区右下角（不占布局列），纯 v-show 包装，
 * 故不建 composable；显隐由 store.vmPanelVisible 驱动。
 */
import { storeToRefs } from "pinia";
import { useVmStore } from "../../stores/vm";

const { vmPanelVisible: visible } = storeToRefs(useVmStore());
</script>

<template>
  <div v-show="visible" class="absolute right-2 bottom-2 z-40 w-104">
    <slot />
  </div>
</template>
