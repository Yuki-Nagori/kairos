<script setup lang="ts">
/** 3D 视口面板：逻辑见 useViewportPanel。悬浮元素（色标 / 视图工具 / 标题 / 坐标）对齐设计稿。
 * 多实例布局：单视口 / 四分格（联动）——任一实例交互后其余实例相机跟随，时间轴全局共享。 */
import UiButton from "../../components/ui/UiButton.vue";
import { useViewportPanel } from "./useViewportPanel";

const {
  layout,
  slotIds,
  attachCanvas,
  slots,
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
  clipAxis,
  clipPosition,
  clipInvert,
  applyClip,
  title,
  centerText,
  loadMesh,
  togglePlay,
  toggleClip,
  resetView,
  zoomBy,
  fitView,
} = useViewportPanel();

/** 悬浮视图工具条：自上而下 放大 / 缩小 / 适应视图 / 复位视角。 */
const VIEW_TOOLS = [
  { id: "zoom-in", icon: "＋", title: "放大", run: () => zoomBy(0.8) },
  { id: "zoom-out", icon: "－", title: "缩小", run: () => zoomBy(1.25) },
  { id: "fit", icon: "⤢", title: "适应视图", run: () => fitView() },
  { id: "reset", icon: "⌂", title: "复位视角", run: () => resetView() },
];
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
      <span v-if="title" class="truncate text-[10px] font-normal text-zinc-500">{{ title }}</span>
      <select
        v-model="layout"
        class="ml-auto rounded border border-zinc-800 bg-zinc-950 px-1.5 py-0.5 text-xs text-zinc-300"
        title="视口布局（联动）"
      >
        <option value="single">单视口</option>
        <option value="quad">四分格（联动）</option>
      </select>
    </div>
    <div
      class="relative flex min-h-0 min-w-0 flex-1 flex-col space-y-3 overflow-x-hidden px-4 py-3.5"
    >
      <div
        class="grid min-h-0 flex-1 gap-2"
        :class="layout === 'quad' ? 'grid-cols-2 grid-rows-2' : 'grid-cols-1'"
      >
        <div
          v-for="id in slotIds"
          :key="id"
          class="relative flex min-h-0 min-w-0"
          :class="layout === 'quad' && id === 0 ? '' : ''"
        >
          <canvas
            :ref="(el) => attachCanvas(id, el)"
            class="h-full w-full rounded-lg bg-zinc-950"
            style="touch-action: none"
          />
          <p
            v-show="!slots[id]!.loaded"
            class="pointer-events-none absolute inset-0 flex items-center justify-center text-xs"
            :class="emptyError ? 'text-red-400' : 'text-zinc-600'"
          >
            {{ emptyText }}
          </p>
          <!-- 悬浮色标图例：数值自上而下 max/mid/min -->
          <div
            v-show="legendVisible"
            class="absolute top-3 left-3 z-10 flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-950/80 px-2 py-1.5"
          >
            <div
              class="h-16 w-2 rounded-sm"
              style="background: linear-gradient(180deg, #f59e0b, #22c55e, #3b82f6)"
            />
            <div class="flex h-16 flex-col justify-between font-mono text-[10px] text-zinc-400">
              <span v-for="(value, index) in legendValues" :key="index">{{
                value.toFixed(2)
              }}</span>
            </div>
          </div>
          <!-- 悬浮视图工具条：放大 / 缩小 / 适应 / 复位（作用于全部联动视口） -->
          <div
            class="absolute top-1/2 right-3 z-10 flex -translate-y-1/2 flex-col gap-0.5 rounded-lg border border-zinc-800 bg-zinc-900/85 p-1"
          >
            <button
              v-for="tool in VIEW_TOOLS"
              :key="tool.id"
              type="button"
              class="grid size-6.5 place-items-center rounded-md text-xs text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
              :title="tool.title"
              @click="tool.run()"
            >
              {{ tool.icon }}
            </button>
          </div>
          <!-- 左下视口标题 / 右下相机注视点读数 -->
          <p
            v-if="title"
            class="pointer-events-none absolute bottom-2.5 left-3 text-[10px] tracking-wide text-zinc-500"
          >
            {{ title }} · 视口 {{ id + 1 }}
          </p>
          <p
            v-show="meshLoaded"
            class="pointer-events-none absolute right-3 bottom-2.5 font-mono text-[10px] text-zinc-500"
          >
            {{ centerText }}
          </p>
        </div>
      </div>
      <div class="flex shrink-0 flex-wrap items-center gap-2">
        <UiButton :disabled="loadDisabled" @click="loadMesh()">载入网格到视口</UiButton>
        <UiButton :disabled="playDisabled" @click="togglePlay()">{{ playLabel }}</UiButton>
        <UiButton :disabled="!meshReady" @click="toggleClip()">
          剖切：{{ clipOn ? "开" : "关" }}
        </UiButton>
        <template v-if="clipOn">
          <select v-model="clipAxis" class="text-xs" @change="applyClip()">
            <option value="x">X</option>
            <option value="y">Y</option>
            <option value="z">Z</option>
          </select>
          <input v-model.number="clipPosition" type="range" min="0" max="1" step="0.01" />
          <label class="flex items-center gap-1 text-xs text-zinc-400">
            <input v-model="clipInvert" type="checkbox" /> 反向
          </label>
        </template>
        <p class="text-xs text-zinc-500">{{ fpsText }}</p>
        <p class="text-xs text-zinc-500">点击模型表面：拾取单元加入探针（XY 图表显示数值）</p>
      </div>
    </div>
  </section>
</template>
