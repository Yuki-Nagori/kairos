<script setup lang="ts">
/**
 * 3D 视口面板：WebGL2 渲染器 + 云图/剖切/时间步动画控制（WebGPU 探测提示）。
 * 视口是工作台主角：卡片弹性充满中列剩余空间，画布随容器缩放。
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { loadField, useAppState } from "../../state";
import type { ScalarField } from "../../types";
import { ViewportRenderer } from "../../render/renderer";
import { detectRenderCapabilityInBrowser } from "../../render/capability";
import { registerSnapshot } from "../../render/snapshot";
import { minMax } from "../../lib/stats";
import { getRenderMesh } from "../../services/geometry";
import UiButton from "../ui/UiButton.vue";

const state = useAppState();

const canvasRef = ref<HTMLCanvasElement | null>(null);

// 空态提示：载入网格前视口不应是一片空白；WebGPU 探测备注与失败文案动态替换。
const emptyText = ref(
  "导入几何并生成网格后，点击「载入网格到视口」查看 3D 模型（WebGPU 可用时自动启用）",
);
const emptyError = ref(false);
const meshLoaded = ref(false);

// 悬浮色标图例：加载场后显示渐变标尺与 max/mid/min（自上而下）。
const legendValues = ref<number[] | null>(null);
const legendVisible = computed(() => legendValues.value !== null);

function updateLegend(field: { values: number[] } | null): void {
  if (field === null || field.values.length === 0) {
    legendValues.value = null;
    return;
  }
  const sorted = [...field.values].sort((a, b) => a - b);
  const max = sorted[sorted.length - 1] ?? 0;
  const mid = sorted[Math.floor(sorted.length / 2)] ?? 0;
  const min = sorted[0] ?? 0;
  legendValues.value = [max, mid, min];
}

// —— 控制条 ——
// 启用条件按数据就绪度推导（原版按钮处于永久禁用的死接线状态，迁移时接通）：
// 载入需已有几何；剖切/重置/播放需网格已上传；播放另需结果时间步目录。
const loadDisabled = computed(() => state.geometries.length === 0);
const meshReady = ref(false);
const playDisabled = computed(() => {
  const catalog = state.resultCatalog;
  return !meshReady.value || catalog === null || catalog.times.length === 0;
});
const fpsText = ref("FPS: —");
const clipOn = ref(false);
const playing = ref(false);
const playLabel = computed(() => (playing.value ? "停止动画" : "播放动画"));

let renderer: ViewportRenderer | null = null;
let renderMesh: { positions: Float32Array; indices: Uint32Array; faceCells: Uint32Array } | null =
  null;
let playTimer: ReturnType<typeof setInterval> | null = null;
let playIndex = 0;

function stopPlay(): void {
  playing.value = false;
  if (playTimer !== null) {
    clearInterval(playTimer);
    playTimer = null;
  }
}

function startPlay(): void {
  updateLegend(state.loadedField);
  const catalog = state.resultCatalog;
  if (catalog === null || catalog.times.length === 0 || renderer === null) {
    return;
  }
  const caseDir = catalog.caseDir;
  const fieldName = state.loadedField?.field ?? "T";
  playing.value = true;
  playIndex = 0;
  playTimer = setInterval(() => {
    if (!playing.value) {
      stopPlay();
      return;
    }
    const step = catalog.times[playIndex % catalog.times.length]!;
    playIndex += 1;
    void loadField(caseDir, step.dirName, fieldName).then(() => {
      applyField(state.loadedField);
    });
  }, 400);
}

function togglePlay(): void {
  if (playing.value) {
    stopPlay();
  } else {
    startPlay();
  }
}

function toggleClip(): void {
  clipOn.value = !clipOn.value;
  renderer?.setClip(clipOn.value, 0);
}

function resetView(): void {
  renderer?.resetView();
}

function ensureRenderer(): void {
  if (renderer !== null) {
    return;
  }
  const canvas = canvasRef.value;
  if (canvas === null) {
    return;
  }
  renderer = ViewportRenderer.create(canvas, (fps) => {
    fpsText.value = `FPS: ${fps}`;
  });
  if (renderer === null) {
    emptyText.value = "当前环境不支持 WebGL2，无法渲染视口。";
    emptyError.value = true;
  }
}

function loadMesh(): void {
  ensureRenderer();
  const geometry = state.geometries[0];
  if (geometry === undefined || renderer === null) {
    return;
  }
  void getRenderMesh(geometry.geometryId).then((data) => {
    renderMesh = {
      positions: new Float32Array(data.positions),
      indices: new Uint32Array(data.indices),
      faceCells: new Uint32Array(data.faceCells),
    };
    renderer?.uploadMesh({
      positions: renderMesh.positions,
      indices: renderMesh.indices,
      faceCells: renderMesh.faceCells,
    });
    meshLoaded.value = true;
    meshReady.value = true;
  });
}

// 场数据加载后自动开启云图着色（值域取自场 min/max），并热更新每面值。
function applyField(field: ScalarField | null): void {
  if (field === null || renderer === null || field.values.length === 0) {
    return;
  }
  if (renderMesh !== null) {
    const perFace = new Float32Array(renderMesh.faceCells.length);
    for (let face = 0; face < perFace.length; face += 1) {
      perFace[face] = field.values[renderMesh.faceCells[face] ?? 0] ?? 0;
    }
    renderer.setFaceValues(perFace);
  }
  const { min, max } = minMax(field.values);
  renderer.setFieldRange(min, max);
}

watch(
  () => state.loadedField,
  (field) => {
    updateLegend(field);
    applyField(field);
  },
);

onMounted(() => {
  const canvas = canvasRef.value;
  if (canvas !== null) {
    registerSnapshot("viewport", canvas);
  }
  void detectRenderCapabilityInBrowser().then((capability) => {
    if (capability.backend === "webgl2") {
      emptyText.value = `${emptyText.value}（${capability.note}）`;
    }
  });
});
onUnmounted(stopPlay);
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
