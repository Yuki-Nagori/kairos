/** XY 图表绘制工具：Canvas 自绘，min-max 抽样保证万点曲线流畅且形状保真。 */
import { themeVar } from "./theme";

interface ChartSeries {
  values: number[];
  color: string;
  label: string;
}

interface ChartRange {
  min: number;
  max: number;
}

/** 计算曲线值域；无有效数据时返回 null，调用方可直接跳过绘制。 */
export function chartRange(series: Array<Pick<ChartSeries, "values">>): ChartRange | null {
  let min = Infinity;
  let max = -Infinity;
  for (const item of series) {
    for (const value of item.values) {
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
  }
  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return null;
  }
  return min === max ? { min: min - 1, max: max + 1 } : { min, max };
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
      // 不变量：index < end <= values.length
      const value = values[index]!;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
    // end >= start+1 恒成立，桶内必有元素，min/max 必然被赋值。
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

  // 默认色来自主题 CSS 变量：深浅主题下面板都保持一致观感。
  ctx.fillStyle = options.background ?? themeVar("--c-bg-input", "#09090b");
  ctx.fillRect(0, 0, width, height);

  let maxPoints = 0;
  for (const seriesItem of series) {
    maxPoints = Math.max(maxPoints, seriesItem.values.length);
  }
  const range = chartRange(series);
  if (!series.length || maxPoints === 0 || range === null) {
    return 0;
  }
  const { min: minValue, max: maxValue } = range;
  const valueSpan = maxValue - minValue;

  // 网格（4×4）
  ctx.strokeStyle = options.gridColor ?? themeVar("--c-grid", "#27272a");
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
      // 不变量：index < sampled.length
      const value = sampled[index]!;
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
  ctx.fillStyle = options.axisLabelColor ?? themeVar("--c-text-muted", "#71717a");
  ctx.font = "11px sans-serif";
  ctx.fillText(options.yLabel ?? "", 8, padding.top + 12);
  ctx.fillText(options.xLabel ?? "", padding.left, height - 8);
  ctx.fillText(minValue.toFixed(3), 8, padding.top + plotHeight);
  ctx.fillText(maxValue.toFixed(3), 8, padding.top + 24);

  return drawn;
}

/** 导出 CSV 内容（含表头）。 */
function csvCell(value: string | number): string {
  const text = String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

/** 导出 RFC 4180 兼容 CSV：BOM 让 Excel 正确识别 UTF-8，CRLF 兼容表格软件。 */
export function toCsv(headers: string[], rows: Array<Array<string | number>>): string {
  const lines = [headers.map(csvCell).join(",")];
  for (const row of rows) {
    lines.push(row.map(csvCell).join(","));
  }
  return `\uFEFF${lines.join("\r\n")}\r\n`;
}
