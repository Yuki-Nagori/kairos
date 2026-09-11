/** XY 图表面板：场分布曲线 / 探针时间曲线 + 探针管理 + CSV 导出。 */
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
import type { ScalarField } from "../../types";
import { drawLineChart } from "../../utils/chart";
import { THEME_CHANGED_EVENT } from "../../composables/useTheme";
import { registerSnapshot } from "../../render/snapshot";

export function useXyChartPanel() {
  const app = useAppStore();
  const results = useResultsStore();

  // 模板 ref 经 useTemplateRef 按名绑定（静态 ref="canvasRef" 不算 setup 变量读取，
  // 解构返回会触发 noUnusedLocals）。
  const canvasRef = useTemplateRef<HTMLCanvasElement>("canvasRef");
  const nodeInput = ref("");
  const working = computed(() => app.busy !== null);

  // 图表模式：空间分布（全场按节点序号）/ 探针时间曲线（探针值随时间步）。
  const mode = ref<"spatial" | "time">("spatial");
  const PROBE_COLORS = ["#34d399", "#fbbf24", "#60a5fa", "#f472b6", "#a78bfa"];

  const canLoadTimeSeries = computed(
    () =>
      results.resultCatalog !== null && results.probes.length > 0 && results.loadedField !== null,
  );

  function loadTimeSeries(): void {
    if (canLoadTimeSeries.value) {
      void results.loadProbeTimeSeries();
    }
  }

  // 时间轴联动：选择时间步 → 加载该步主场 → 视口云图与探针数值同步刷新。
  const selectedTimeDir = ref("");
  function jumpToTime(): void {
    const catalog = results.resultCatalog;
    const fieldName = results.probeSeriesField;
    if (catalog === null || fieldName === null || selectedTimeDir.value === "") {
      return;
    }
    void results.loadField(catalog.caseDir, selectedTimeDir.value, fieldName);
  }

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
    if (mode.value === "time") {
      drawTimeMode(ctx, canvas);
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

    // 探针点绘制在曲线上（圆点 + 值标注）；padding 需与 drawLineChart 的留白一致。
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

  /** 时间曲线模式：每探针一条曲线，x 为目录时间步序。 */
  function drawTimeMode(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement): void {
    const series = results.probeTimeSeries.map((entry, index) => ({
      values: entry.samples.map((sample) => sample.value),
      color: PROBE_COLORS[index % PROBE_COLORS.length]!,
      label: `#${entry.nodeIndex}`,
    }));
    void drawLineChart(ctx, series, {
      width: canvas.width,
      height: canvas.height,
      xLabel: "时间步（序）",
      yLabel: results.probeSeriesField ?? "",
    });
  }

  watch(
    () => [results.loadedField, results.probes, results.probeTimeSeries, mode.value],
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
    mode,
    canLoadTimeSeries,
    loadTimeSeries,
    selectedTimeDir,
    jumpToTime,
    addProbeFromInput,
    draw,
  };
}
