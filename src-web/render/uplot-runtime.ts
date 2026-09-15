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

export function resetUplot(plot: uPlot | null, current: string, next: string): uPlot | null {
  if (plot !== null && current !== next) {
    plot.destroy();
    return null;
  }
  return plot;
}

function observeUplotSize(plot: uPlot, host: HTMLDivElement): () => void {
  if (typeof ResizeObserver !== "function") {
    return () => undefined;
  }
  const resize = () => {
    const width = Math.max(Math.floor(host.getBoundingClientRect().width), 320);
    plot.setSize({ width, height: 200 });
  };
  const observer = new ResizeObserver(resize);
  observer.observe(host);
  resize();
  return () => observer.disconnect();
}

export function observeUplotSizeIfPresent(plot: uPlot | null, host: HTMLDivElement): () => void {
  return plot === null ? () => undefined : observeUplotSize(plot, host);
}

export function resetUplotZoom(plot: uPlot | null): void {
  if (plot === null) {
    return;
  }
  const auto = { min: null, max: null } as unknown as { min: number; max: number };
  plot.setScale("x", auto);
  plot.setScale("y", auto);
}
