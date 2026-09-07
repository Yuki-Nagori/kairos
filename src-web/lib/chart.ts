/** XY 图表绘制工具：Canvas 自绘，min-max 抽样保证万点曲线流畅且形状保真。 */

interface ChartSeries {
  values: number[];
  color: string;
  label: string;
}

/** min-max 抽样：每桶保留最小/最大值，长度变为 2 × 桶数（保形降采样）。 */
export function downsampleSeries(values: number[], targetBuckets: number): number[] {
  if (targetBuckets <= 0 || values.length <= targetBuckets * 2) {
    return [...values];
  }
  const out: number[] = [];
  const bucketSize = values.length / targetBuckets;
  for (let bucket = 0; bucket < targetBuckets; bucket += 1) {
    const start = Math.floor(bucket * bucketSize);
    const end = Math.min(values.length, Math.max(start + 1, Math.floor((bucket + 1) * bucketSize)));
    let min = Infinity;
    let max = -Infinity;
    for (let index = start; index < end; index += 1) {
      const value = values[index] ?? 0;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
    if (min === Infinity) {
      // 空桶（极端bucketSize）—— 取起点值
      const fallback = values[start] ?? 0;
      out.push(fallback, fallback);
      continue;
    }
    out.push(min, max);
  }
  return out;
}

interface DrawOptions {
  width: number;
  height: number;
  background?: string;
  gridColor?: string;
  axisLabelColor?: string;
  xLabel?: string;
  yLabel?: string;
}

/** 在 Canvas 2D 上绘制多条曲线（自动量程、网格、坐标轴标签）。返回实际绘制的点数。 */
export function drawLineChart(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
  options: DrawOptions,
): number {
  const { width, height } = options;
  const padding = { left: 56, right: 16, top: 16, bottom: 36 };
  const plotWidth = Math.max(width - padding.left - padding.right, 1);
  const plotHeight = Math.max(height - padding.top - padding.bottom, 1);

  ctx.fillStyle = options.background ?? "#09090b";
  ctx.fillRect(0, 0, width, height);

  let minValue = Infinity;
  let maxValue = -Infinity;
  let maxPoints = 0;
  for (const seriesItem of series) {
    for (const value of seriesItem.values) {
      minValue = Math.min(minValue, value);
      maxValue = Math.max(maxValue, value);
    }
    maxPoints = Math.max(maxPoints, seriesItem.values.length);
  }
  if (
    !series.length ||
    maxPoints === 0 ||
    !Number.isFinite(minValue) ||
    !Number.isFinite(maxValue)
  ) {
    return 0;
  }
  if (minValue === maxValue) {
    minValue -= 1;
    maxValue += 1;
  }
  const valueSpan = maxValue - minValue;

  // 网格（4×4）
  ctx.strokeStyle = options.gridColor ?? "#27272a";
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let grid = 1; grid < 4; grid += 1) {
    const gx = padding.left + (plotWidth * grid) / 4;
    ctx.moveTo(gx, padding.top);
    ctx.lineTo(gx, padding.top + plotHeight);
    const gy = padding.top + (plotHeight * grid) / 4;
    ctx.moveTo(padding.left, gy);
    ctx.lineTo(padding.left + plotWidth, gy);
  }
  ctx.stroke();

  let drawn = 0;
  for (const seriesItem of series) {
    if (seriesItem.values.length === 0) {
      continue;
    }
    const buckets = Math.max(Math.min(seriesItem.values.length, Math.floor(plotWidth)), 2);
    const sampled = downsampleSeries(seriesItem.values, buckets);
    ctx.strokeStyle = seriesItem.color;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let index = 0; index < sampled.length; index += 1) {
      const value = sampled[index] ?? 0;
      const x = padding.left + (index * plotWidth) / Math.max(sampled.length - 1, 1);
      const y = padding.top + plotHeight - ((value - minValue) / valueSpan) * plotHeight;
      if (index === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
      drawn += 1;
    }
    ctx.stroke();
  }

  // 坐标轴标签
  ctx.fillStyle = options.axisLabelColor ?? "#71717a";
  ctx.font = "11px sans-serif";
  ctx.fillText(options.yLabel ?? "", 8, padding.top + 12);
  ctx.fillText(options.xLabel ?? "", padding.left, height - 8);
  ctx.fillText(minValue.toFixed(3), 8, padding.top + plotHeight);
  ctx.fillText(maxValue.toFixed(3), 8, padding.top + 24);

  return drawn;
}

/** 导出 CSV 内容（含表头）。 */
export function toCsv(headers: string[], rows: Array<number[]>): string {
  const lines = [headers.join(",")];
  for (const row of rows) {
    lines.push(row.map((value) => String(value)).join(","));
  }
  return lines.join("\n");
}
