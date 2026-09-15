/** uPlot 适配边界：只承载通用图表配置，结果场与探针语义仍由面板编排。 */
import type uPlot from "uplot";
import { themeVar } from "../../utils/theme";

export type UplotSeries = {
  label: string;
  stroke: string;
  points?: { show: boolean; size: number };
};

export function uplotOptions(
  width: number,
  height: number,
  series: UplotSeries[],
  labels: { x: string; y: string } = { x: "序号", y: "值" },
): uPlot.Options {
  return {
    width,
    height,
    scales: { x: { time: false }, y: { auto: true } },
    series: [{ label: "序号" }, ...series],
    axes: [
      {
        label: labels.x,
        stroke: themeVar("--c-text-muted", "#71717a"),
        grid: { stroke: themeVar("--c-grid", "#27272a"), width: 1 },
      },
      {
        label: labels.y,
        stroke: themeVar("--c-text-muted", "#71717a"),
        grid: { stroke: themeVar("--c-grid", "#27272a"), width: 1 },
      },
    ],
    cursor: { drag: { x: true, y: false } },
  };
}
