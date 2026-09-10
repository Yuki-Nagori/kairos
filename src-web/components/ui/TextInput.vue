<script setup lang="ts">
/**
 * 文本 / 数字输入框：v-model 双向绑定，type 区分 text 与 number。
 * 约束属性（min/step 等）与宽度类由调用方经 attrs 透传补充。
 */
withDefaults(
  defineProps<{
    modelValue: string;
    type?: "text" | "number";
    placeholder?: string;
  }>(),
  { type: "text", placeholder: "" },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <input
    :type="type"
    :value="modelValue"
    :placeholder="placeholder"
    class="rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-100 transition-colors placeholder:text-zinc-600 focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20 focus:outline-none"
    @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
