/** uPlot 适配边界：只承载通用图表配置，结果场与探针语义仍由面板编排。 */
import type uPlot from "uplot";

type UplotSeries = { label: string; stroke: string };

export function uplotOptions(width: number, height: number, series: UplotSeries[]): uPlot.Options {
  return {
    width,
    height,
    scales: { x: { time: false }, y: { auto: true } },
    series: [{ label: "序号" }, ...series],
    axes: [{ label: "序号" }, { label: "值" }],
    cursor: { drag: { x: true, y: false } },
  };
}
