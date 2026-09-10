<script setup lang="ts">
/**
 * 终端窗口图标：内联 SVG 描边继承 currentColor（<img> 图片做不到），深浅主题自动跟随。
 * 图标来自 Icons8（免费授权要求署名，见 README「致谢」）；不得用作产品 Logo 或单独再分发。
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
