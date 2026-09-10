<script setup lang="ts">
/**
 * 终端窗口图标：内联 SVG 描边继承 currentColor，深浅主题自动跟随
 * （替代旧的 <img> 引用方案——图片无法继承颜色）。
 *
 * 来源：Icons8（icons8.com）的终端图标——免费授权要求使用时署名，
 * 已在 README「致谢」标注；不得用作产品 Logo 或单独再分发图标文件。
 */
</script>

<template>
  <svg
    viewBox="0 0 48 48"
    fill="none"
    stroke="currentColor"
    stroke-width="3"
    stroke-linejoin="round"
    stroke-linecap="round"
    aria-hidden="true"
    class="inline-block"
  >
    <path
      d="M35.5,40.5h-23 c-2.761,0-5-2.239-5-5v-23c0-2.761,2.239-5,5-5h23c2.761,0,5,2.239,5,5v23C40.5,38.261,38.261,40.5,35.5,40.5z"
    />
    <line x1="40.5" y1="15.5" x2="7.5" y2="15.5" />
    <polyline points="17.5,23.5 21.5,27.5 17.5,31.5" />
    <line x1="25.5" y1="31.5" x2="31.5" y2="31.5" />
  </svg>
</template>
