import { appStore, addProbe, removeProbe, exportFieldCsv } from "../state";
import type { ScalarField } from "../types";
import { drawLineChart } from "../lib/chart";
import { THEME_CHANGED_EVENT } from "../theme";
import { registerSnapshot } from "../render/snapshot";
import { button, card, hint, textInput } from "./ui";

/** XY 图表面板：场分布曲线（节点序号 → 值）+ 探针管理 + CSV 导出。 */
export function createXyChartPanel(): HTMLElement {
  const { root, body } = card("XY 曲线与探针");

  const canvas = document.createElement("canvas");
  canvas.width = 720;
  canvas.height = 260;
  canvas.className = "w-full rounded-lg bg-zinc-950";
  registerSnapshot("xy-chart", canvas);

  const probeForm = document.createElement("div");
  probeForm.className = "flex flex-wrap items-center gap-2";
  const nodeInput = textInput("节点序号", "w-28");
  const addProbeButton = button("添加探针");
  const exportButton = button("导出 CSV");
  const refreshButton = button("重绘");
  probeForm.append(nodeInput, addProbeButton, exportButton, refreshButton);

  const probeList = document.createElement("div");
  probeList.className = "space-y-1";

  addProbeButton.addEventListener("click", () => {
    const nodeIndex = Number(nodeInput.value);
    addProbe(nodeIndex);
    nodeInput.value = "";
  });
  refreshButton.addEventListener("click", () => appStore.set({}));
  exportButton.addEventListener("click", () => exportFieldCsv());

  function render(): void {
    const { loadedField, probes, busy } = appStore.get();
    const working = busy !== null;
    addProbeButton.disabled = working;
    exportButton.disabled = working;
    nodeInput.disabled = working;

    const ctx = canvas.getContext("2d");
    if (ctx === null) {
      return;
    }
    const series: Array<{ values: number[]; color: string; label: string }> = [];
    const field: ScalarField | null = loadedField;
    if (field !== null && field.values.length > 0) {
      series.push({ values: field.values, color: "#34d399", label: field.field });
    }
    // 探针位置的值叠加为参考线（取值点单独绘制）。
    const probeDots = probes.map((probe) => {
      const value = field?.values[probe.nodeIndex] ?? 0;
      return { probe, value };
    });

    const drawn = drawLineChart(ctx, series, {
      width: canvas.width,
      height: canvas.height,
      xLabel: field !== null ? "节点序号" : "",
      yLabel: field?.field ?? "",
    });

    // 探针点在曲线上标注（简单竖线 + 值）
    const ctx2d = canvas.getContext("2d");
    if (ctx2d !== null && field !== null && field.values.length > 0) {
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
      for (const { probe, value } of probeDots) {
        const x =
          padding.left + (probe.nodeIndex / Math.max(field.values.length - 1, 1)) * plotWidth;
        const y = 16 + plotHeight - ((value - minValue) / span) * plotHeight;
        ctx2d.fillStyle = "#fbbf24";
        ctx2d.beginPath();
        ctx2d.arc(x, y, 4, 0, Math.PI * 2);
        ctx2d.fill();
        ctx2d.fillStyle = "#fbbf24";
        ctx2d.fillText(
          `#${probe.nodeIndex}: ${value.toFixed(2)}`,
          Math.min(x + 6, canvas.width - 90),
          y - 6,
        );
      }
    }

    probeList.replaceChildren();
    if (probes.length === 0) {
      probeList.append(hint("暂无探针。输入节点序号后添加。"));
    } else {
      for (const probe of probes) {
        const row = document.createElement("div");
        row.className =
          "flex items-center justify-between rounded border border-zinc-800 px-2 py-1";
        const label = document.createElement("span");
        label.className = "text-zinc-300";
        label.textContent = `探针 #${probe.id} · 节点 ${probe.nodeIndex}`;
        const remove = button("✕", "danger");
        remove.addEventListener("click", () => removeProbe(probe.id));
        row.append(label, remove);
        probeList.append(row);
      }
    }
    void drawn;
  }

  body.append(canvas, probeForm, probeList);
  render();
  // 主题切换后画布配色取自 CSS 变量，需整帧重绘。
  window.addEventListener(THEME_CHANGED_EVENT, render);
  appStore.subscribe(render);
  return root;
}
