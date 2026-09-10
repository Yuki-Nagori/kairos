<script setup lang="ts">
// 应用根组件：布局组合在这里完成。
// 各列面板通过 <KeepAlive> + v-show 按阶段显隐。
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
    <slot />
  </div>
</template>
