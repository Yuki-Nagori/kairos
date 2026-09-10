<script setup lang="ts">
/** 3D 视口面板：逻辑见 useViewportPanel。 */
import UiButton from "../../components/ui/UiButton.vue";
import { useViewportPanel } from "./useViewportPanel";

const {
  emptyText,
  emptyError,
  meshLoaded,
  legendVisible,
  legendValues,
  loadDisabled,
  meshReady,
  playDisabled,
  playLabel,
  fpsText,
  clipOn,
  loadMesh,
  togglePlay,
  toggleClip,
  resetView,
} = useViewportPanel();
</script>

<template>
  <!-- 卡片结构内联：root/body 需要追加 flex 撑满中列的类，超出通用 Card 的插槽能力 -->
  <section
    class="flex min-h-70 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900"
  >
    <div
      class="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/40 px-4 py-2.5 text-xs font-semibold tracking-wide text-zinc-200 select-none"
    >
      <h2>3D 视口</h2>
    </div>
    <div class="flex min-h-0 min-w-0 flex-1 flex-col space-y-3 overflow-x-hidden px-4 py-3.5">
      <div class="relative flex min-h-0 flex-1">
        <canvas
          ref="canvasRef"
          class="h-full w-full rounded-lg bg-zinc-950"
          style="touch-action: none"
        />
        <p
          v-show="!meshLoaded"
          class="pointer-events-none absolute inset-0 flex items-center justify-center text-xs"
          :class="emptyError ? 'text-red-400' : 'text-zinc-600'"
        >
          {{ emptyText }}
        </p>
        <div
          v-show="legendVisible"
          class="absolute top-3 left-3 z-10 flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-950/80 px-2 py-1.5"
        >
          <div
            class="h-16 w-2 rounded-sm"
            style="background: linear-gradient(180deg, #f59e0b, #22c55e, #3b82f6)"
          />
          <div class="flex h-16 flex-col justify-between font-mono text-[10px] text-zinc-400">
            <span v-for="(value, index) in legendValues" :key="index">{{ value.toFixed(2) }}</span>
          </div>
        </div>
      </div>
      <div class="flex shrink-0 flex-wrap items-center gap-2">
        <UiButton :disabled="loadDisabled" @click="loadMesh()">载入网格到视口</UiButton>
        <UiButton :disabled="playDisabled" @click="togglePlay()">{{ playLabel }}</UiButton>
        <UiButton :disabled="!meshReady" @click="toggleClip()">
          剖切：{{ clipOn ? "开" : "关" }}
        </UiButton>
        <UiButton :disabled="!meshReady" @click="resetView()">重置视角</UiButton>
        <p class="text-xs text-zinc-500">{{ fpsText }}</p>
      </div>
    </div>
  </section>
</template>
