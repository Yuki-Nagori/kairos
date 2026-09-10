<script setup lang="ts">
/**
 * 面板内紧凑下拉框：v-model 绑定选中值，<option> 由调用方经插槽填充（含动态刷新场景）。
 */
defineProps<{ modelValue: string }>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <select
    :value="modelValue"
    class="rounded-lg border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs text-zinc-100 transition-colors focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20 focus:outline-none"
    @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
  >
    <slot />
  </select>
</template>
