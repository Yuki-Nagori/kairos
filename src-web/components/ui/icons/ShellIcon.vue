<script setup lang="ts">
/**
 * 终端窗口图标：24 网格自绘几何（圆角窗体 + 标题栏分隔线 + 提示符箭头 + 光标），
 * 描边继承 currentColor 深浅主题自动跟随。
 */
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    class="inline-block"
  >
    <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" />
    <path d="M3.5 9.5h17" />
    <path d="M8 13.5l3 2.5-3 2.5" />
    <path d="M13.5 18.5H17" />
  </svg>
</template>
