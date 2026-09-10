<script setup lang="ts">
/**
 * LaTeX 公式渲染（KaTeX）：renderToString 生成 HTML，v-html 注入。
 * throwOnError 关闭以保证个别公式错误不炸整个 UI；内容全部来自
 * katex 输出（非用户 HTML），颜色经 currentColor 随主题。
 */
import { computed } from "vue";
import katex from "katex";
import "katex/dist/katex.min.css";

const props = withDefaults(
  defineProps<{
    tex: string;
    displayMode?: boolean;
  }>(),
  { displayMode: true },
);

const html = computed(() =>
  katex.renderToString(props.tex, { displayMode: props.displayMode, throwOnError: false }),
);
</script>

<template>
  <div
    :class="{ 'overflow-x-auto text-center text-[11px] text-zinc-300': displayMode }"
    v-html="html"
  />
</template>
