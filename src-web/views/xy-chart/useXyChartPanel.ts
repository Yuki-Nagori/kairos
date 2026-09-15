/** XY 图表面板：场分布曲线 / 探针时间曲线 + 探针管理 + CSV 导出。 */
import { computed, onMounted, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import type uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
import { THEME_CHANGED_EVENT } from "../../utils/theme";
import { createUplot, observeUplotSizeIfPresent, resetUplot } from "../../render/uplot-runtime";

export function useXyChartPanel() {
  const app = useAppStore();
  const results = useResultsStore();

  const plotHostRef = useTemplateRef<HTMLDivElement>("plotHostRef");
  const plot = shallowRef<uPlot | null>(null);
  const plotSignature = ref("");
  let stopResizeObserver: () => void = () => undefined;
  const nodeInput = ref("");
  const working = computed(() => app.working);

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

  function plotSeries() {
    if (mode.value === "time") {
      return results.probeTimeSeries.map((entry, index) => ({
        label: `#${entry.nodeIndex}`,
        stroke: PROBE_COLORS[index % PROBE_COLORS.length]!,
      }));
    }
    const field = results.loadedField;
    return [
      { label: field?.field ?? "场值", stroke: "#34d399" },
      ...results.probes.map((probe, index) => ({
        label: `探针 #${probe.nodeIndex}`,
        stroke: PROBE_COLORS[(index + 1) % PROBE_COLORS.length]!,
        points: { show: true, size: 6 },
      })),
    ];
  }

  function plotData(): uPlot.AlignedData {
    if (mode.value === "time") {
      const series = results.probeTimeSeries;
      const length = series[0]?.samples.length ?? 0;
      return [
        Array.from({ length }, (_, index) => index),
        ...series.map((entry) => entry.samples.map((sample) => sample.value)),
      ];
    }
    const values = results.loadedField?.values ?? [];
    const probes = results.probes.map((probe) =>
      values.map((value, index) => (index === probe.nodeIndex ? value : Number.NaN)),
    );
    return [Array.from({ length: values.length }, (_, index) => index), values, ...probes];
  }

  function ensurePlot() {
    if (plot.value !== null || plotHostRef.value === null) {
      return plot.value;
    }
    // happy-dom 与部分嵌入式 WebView 没有 Path2D；保留 Canvas 降级路径，避免异步绘制抛错。
    plot.value = createUplot(plotHostRef.value, plotData(), plotSeries());
    plotSignature.value = `${mode.value}:${results.probes.map((probe) => probe.id).join(",")}`;
    stopResizeObserver();
    stopResizeObserver = observeUplotSizeIfPresent(plot.value, plotHostRef.value);
    return plot.value;
  }

  function draw(): void {
    const nextSignature = `${mode.value}:${results.probes.map((probe) => probe.id).join(",")}`;
    plot.value = resetUplot(plot.value, plotSignature.value, nextSignature);
    plotSignature.value = nextSignature;
    ensurePlot()?.setData(plotData());
  }

  watch(
    () => [results.loadedField, results.probes, results.probeTimeSeries, mode.value],
    () => draw(),
  );

  onMounted(() => {
    ensurePlot();
    // 主题切换后画布配色取自 CSS 变量，需整帧重绘。
    window.addEventListener(THEME_CHANGED_EVENT, draw);
    draw();
  });
  onUnmounted(() => {
    window.removeEventListener(THEME_CHANGED_EVENT, draw);
    stopResizeObserver();
    plot.value?.destroy();
    plot.value = null;
  });

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
    plotHostRef,
  };
}
