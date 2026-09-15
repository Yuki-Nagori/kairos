/**
 * XY 图表数据处理基准（不依赖浏览器 Canvas）。
 * 运行：bun src-web/bench/chart.ts
 * 口径：100 万点单曲线的值域扫描与 min-max 抽样；用于候选图表库引入前的基线。
 */
import { Bench } from "tinybench";
import { chartRange, downsampleSeries } from "../utils/chart-data";

const values = Array.from(
  { length: 1_000_000 },
  (_, index) => Math.sin(index / 37) + Math.cos(index / 101) * 0.25,
);
const bench = new Bench({ warmupIterations: 10, iterations: 100 });

bench
  .add("chartRange: 1M values", () => {
    chartRange([{ values }]);
  })
  .add("downsampleSeries: 1M → 2K", () => {
    downsampleSeries(values, 1_000);
  });

const tasks = await bench.run();
for (const task of tasks) {
  if (task.result.state !== "completed") {
    throw new Error(`基准未完成：${task.name}（${task.result.state}）`);
  }
  const nsPerOp = task.result.latency.mean * 1_000_000;
  console.log(`${task.name}: ${nsPerOp.toFixed(1)} ns/op`);
}
