/** uPlot 适配边界：只承载通用图表配置，结果场与探针语义仍由面板编排。 */
import type uPlot from "uplot";
import { themeVar } from "../../utils/theme";

export type UplotSeries = {
  label: string;
  stroke: string;
  points?: { show: boolean; size: number };
};

export function uplotOptions(width: number, height: number, series: UplotSeries[]): uPlot.Options {
  return {
    width,
    height,
    scales: { x: { time: false }, y: { auto: true } },
    series: [{ label: "序号" }, ...series],
    axes: [
      {
        label: "序号",
        stroke: themeVar("--c-text-muted", "#71717a"),
        grid: { stroke: themeVar("--c-grid", "#27272a"), width: 1 },
      },
      {
        label: "值",
        stroke: themeVar("--c-text-muted", "#71717a"),
        grid: { stroke: themeVar("--c-grid", "#27272a"), width: 1 },
      },
    ],
    cursor: { drag: { x: true, y: false } },
  };
}
