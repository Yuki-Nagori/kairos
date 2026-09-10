<script setup lang="ts">
/**
 * LaTeX 公式渲染（KaTeX）：render 直接构建 DOM 节点挂进容器，
 * 不经 HTML 字符串解析，从结构上消除注入面；个别公式错误经
 * throwOnError 关闭不炸 UI。颜色经 currentColor 随主题。
 */
import { onMounted, ref, watch } from "vue";
import katex from "katex";
import "katex/dist/katex.min.css";

const props = withDefaults(
  defineProps<{
    tex: string;
    displayMode?: boolean;
  }>(),
  { displayMode: true },
);

const container = ref<HTMLDivElement | null>(null);

function render(): void {
  if (container.value !== null) {
    katex.render(props.tex, container.value, {
      displayMode: props.displayMode,
      throwOnError: false,
    });
  }
}

onMounted(render);
watch([() => props.tex, () => props.displayMode], render);
</script>

<template>
  <div
    ref="container"
    :class="{ 'overflow-x-auto text-center text-[11px] text-zinc-300': displayMode }"
  />
</template>
