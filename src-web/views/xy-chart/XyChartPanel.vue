<script setup lang="ts">
/** XY 图表面板：场分布曲线（节点序号 → 值）+ 探针管理 + CSV 导出。 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { addProbe, exportFieldCsv, removeProbe, useAppState } from "../../state";
import type { ScalarField } from "../../types";
import { drawLineChart } from "../../utils/chart";
import { THEME_CHANGED_EVENT } from "../../theme";
import { registerSnapshot } from "../../render/snapshot";
import UiButton from "../../components/ui/UiButton.vue";
import UiTextInput from "../../components/ui/UiTextInput.vue";
import Card from "../../components/ui/UiCard.vue";

const state = useAppState();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const nodeInput = ref("");
const working = computed(() => state.busy !== null);

function addProbeFromInput(): void {
  addProbe(Number(nodeInput.value));
  nodeInput.value = "";
}

// 探针位置的值叠加为参考线（取值点单独绘制在曲线上）。
const probeDots = computed(() =>
  state.probes.map((probe) => {
    const value = state.loadedField?.values[probe.nodeIndex] ?? 0;
    return { probe, value };
  }),
);

function draw(): void {
  const canvas = canvasRef.value;
  if (canvas === null) {
    return;
  }
  const ctx = canvas.getContext("2d");
  if (ctx === null) {
    return;
  }
  const series: Array<{ values: number[]; color: string; label: string }> = [];
  const field: ScalarField | null = state.loadedField;
  if (field !== null && field.values.length > 0) {
    series.push({ values: field.values, color: "#34d399", label: field.field });
  }

  void drawLineChart(ctx, series, {
    width: canvas.width,
    height: canvas.height,
    xLabel: field !== null ? "节点序号" : "",
    yLabel: field?.field ?? "",
  });

  // 探针点在曲线上标注（简单竖线 + 值）。
  if (field !== null && field.values.length > 0) {
    const padding = { left: 56, top: 16 };
    const plotWidth = canvas.width - padding.left - 16;
    let minValue = Infinity;
    let maxValue = -Infinity;
    for (const value of field.values) {
      minValue = Math.min(minValue, value);
      maxValue = Math.max(maxValue, value);
    }
    const span = Math.max(maxValue - minValue, 1e-9);
    const plotHeight = canvas.height - 16 - 36;
    for (const { probe, value } of probeDots.value) {
      const x = padding.left + (probe.nodeIndex / Math.max(field.values.length - 1, 1)) * plotWidth;
      const y = 16 + plotHeight - ((value - minValue) / span) * plotHeight;
      ctx.fillStyle = "#fbbf24";
      ctx.beginPath();
      ctx.arc(x, y, 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillText(
        `#${probe.nodeIndex}: ${value.toFixed(2)}`,
        Math.min(x + 6, canvas.width - 90),
        y - 6,
      );
    }
  }
}

watch(
  () => [state.loadedField, state.probes],
  () => draw(),
);

onMounted(() => {
  const canvas = canvasRef.value;
  if (canvas !== null) {
    registerSnapshot("xy-chart", canvas);
  }
  // 主题切换后画布配色取自 CSS 变量，需整帧重绘。
  window.addEventListener(THEME_CHANGED_EVENT, draw);
  draw();
});
onUnmounted(() => window.removeEventListener(THEME_CHANGED_EVENT, draw));
</script>

<template>
  <Card class="shrink-0" title="XY 曲线与探针">
    <canvas ref="canvasRef" width="720" height="200" class="w-full rounded-lg bg-zinc-950" />
    <div class="flex flex-wrap items-center gap-2">
      <UiTextInput v-model="nodeInput" placeholder="节点序号" class="w-28" :disabled="working" />
      <UiButton :disabled="working" @click="addProbeFromInput()">添加探针</UiButton>
      <UiButton :disabled="working" @click="exportFieldCsv()">导出 CSV</UiButton>
      <UiButton @click="draw()">重绘</UiButton>
    </div>
    <div class="space-y-1">
      <p v-if="state.probes.length === 0" class="text-xs text-zinc-500">
        暂无探针。输入节点序号后添加。
      </p>
      <template v-else>
        <div
          v-for="probe in state.probes"
          :key="probe.id"
          class="flex items-center justify-between rounded border border-zinc-800 px-2 py-1"
        >
          <span class="text-zinc-300">探针 #{{ probe.id }} · 节点 {{ probe.nodeIndex }}</span>
          <UiButton variant="danger" @click="removeProbe(probe.id)">✕</UiButton>
        </div>
      </template>
    </div>
  </Card>
</template>
