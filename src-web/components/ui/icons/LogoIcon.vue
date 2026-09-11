<script setup lang="ts">
/** 品牌 Logo 图标：沙漏 + K 字母的自绘简化标记（title bar / 面板头部用）。 */
defineProps<{ size?: number }>();
</script>

<template>
  <svg
    viewBox="0 0 128 128"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
    :width="size ?? 16"
    :height="size ?? 16"
  >
    <!-- 沙漏外框 -->
    <path
      d="M36 28 L92 28 L80 64 L92 100 L36 100 L48 64 L36 28 Z"
      stroke="#34d399"
      stroke-width="6"
      stroke-linejoin="round"
    />
    <!-- 上腔流动曲线 -->
    <path d="M46 36 Q64 48 82 36" stroke="#34d399" stroke-width="6" stroke-linecap="round" />
    <!-- 下腔流动曲线 -->
    <path d="M46 92 Q64 80 82 92" stroke="#f59e0b" stroke-width="6" stroke-linecap="round" />
    <!-- 颈部光柱 -->
    <rect x="58" y="54" width="12" height="20" rx="6" fill="#f59e0b" />
    <!-- K 字母笔画 -->
    <path
      d="M44 44 L56 64 L44 84"
      stroke="#34d399"
      stroke-width="5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <path d="M56 64 L80 84" stroke="#34d399" stroke-width="5" stroke-linecap="round" />
  </svg>
</template>
