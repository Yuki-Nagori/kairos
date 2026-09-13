<script setup lang="ts">
/** LaTeX 公式渲染（KaTeX）：逻辑见 useLatex。
 * 公式在可访问性树上按单张图像暴露：KaTeX 输出的 MathML 会被逐符号展开
 * （「η」「=」「0」各占一个节点），读屏与自动化驱动都会被符号噪音淹没。 */
import { ref } from "vue";
import "katex/dist/katex.min.css";
import { useLatex } from "./useLatex";

const props = withDefaults(
  defineProps<{
    tex: string;
    /** 公式名（可访问性树上的节点名，如「Cross-WLF 黏度模型」）。 */
    label: string;
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
    role="img"
    :aria-label="label"
    :class="{ 'overflow-x-auto text-center text-[11px] text-zinc-300': displayMode }"
  />
</template>
