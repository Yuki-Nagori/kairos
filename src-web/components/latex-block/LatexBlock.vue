<script setup lang="ts">
/** LaTeX 公式渲染（KaTeX）：逻辑见 useLatex。 */
import { ref } from "vue";
import "katex/dist/katex.min.css";
import { useLatex } from "./useLatex";

const props = withDefaults(
  defineProps<{
    tex: string;
    displayMode?: boolean;
  }>(),
  { displayMode: true },
);

// 容器引用在此声明并传入 useLatex：静态 ref 字符串在运行时经 setupState 解析到它。
const container = ref<HTMLDivElement | null>(null);

useLatex(
  container,
  () => props.tex,
  () => props.displayMode,
);
</script>

<template>
  <div
    ref="container"
    :class="{ 'overflow-x-auto text-center text-[11px] text-zinc-300': displayMode }"
  />
</template>
