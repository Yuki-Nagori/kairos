/** uPlot 运行时桥接；Canvas/WebView 细节隔离在渲染层，不进入逻辑覆盖率口径。 */
import uPlot from "uplot";
import { registerSnapshot } from "./snapshot";
import { uplotOptions, type UplotSeries } from "../views/xy-chart/uplot-adapter";

export function createUplot(
  host: HTMLDivElement,
  data: uPlot.AlignedData,
  series: UplotSeries[],
): uPlot | null {
  if (typeof Path2D !== "function") {
    return null;
  }
  try {
    const plot = new uPlot(uplotOptions(720, 200, series), data, host);
    plot.ctx.canvas.classList.add("w-full", "rounded-lg", "bg-zinc-950");
    registerSnapshot("xy-chart", plot.ctx.canvas);
    return plot;
  } catch {
    return null;
  }
}
