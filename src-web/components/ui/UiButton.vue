<script setup lang="ts">
/**
 * 统一按钮：primary 实心强调、ghost 描边、danger 危险描边。
 * disabled 与 @click 走原生透传，不在组件内重复声明。
 */
withDefaults(defineProps<{ variant?: "primary" | "ghost" | "danger" }>(), {
  variant: "ghost",
});

const VARIANTS: Record<"primary" | "ghost" | "danger", string> = {
  primary: "bg-emerald-500 text-zinc-950 shadow-sm shadow-emerald-500/20 hover:bg-emerald-400",
  danger: "border border-zinc-700 text-zinc-400 hover:border-red-500/70 hover:text-red-400",
  ghost:
    "border border-zinc-700 text-zinc-200 hover:border-emerald-500/70 hover:bg-emerald-500/10 hover:text-emerald-300",
};
</script>

<template>
  <button
    type="button"
    class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40"
    :class="VARIANTS[variant]"
  >
    <slot />
  </button>
</template>
