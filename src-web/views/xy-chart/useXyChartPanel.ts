import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
import type { ScalarField } from "../../types";
import { drawLineChart } from "../../utils/chart";
import { THEME_CHANGED_EVENT } from "../../composables/useTheme";
import { registerSnapshot } from "../../render/snapshot";

/** XY 图表面板逻辑：场分布曲线（节点序号 → 值）+ 探针管理 + CSV 导出。 */
export function useXyChartPanel() {
  const app = useAppStore();
  const results = useResultsStore();

  // 模板 ref 经 useTemplateRef 按名绑定（静态 ref="canvasRef" 不算 setup 变量读取，
  // 解构返回会触发 noUnusedLocals）。
  const canvasRef = useTemplateRef<HTMLCanvasElement>("canvasRef");
  const nodeInput = ref("");
  const working = computed(() => app.busy !== null);

  function addProbeFromInput(): void {
    results.addProbe(Number(nodeInput.value));
    nodeInput.value = "";
  }

  // 探针位置的值叠加为参考线（取值点单独绘制在曲线上）。
  const probeDots = computed(() =>
    results.probes.map((probe) => {
      const value = results.loadedField?.values[probe.nodeIndex] ?? 0;
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
    const field: ScalarField | null = results.loadedField;
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
        const x =
          padding.left + (probe.nodeIndex / Math.max(field.values.length - 1, 1)) * plotWidth;
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
    () => [results.loadedField, results.probes],
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

  return {
    nodeInput,
    working,
    probeDots,
    addProbeFromInput,
    draw,
  };
}
