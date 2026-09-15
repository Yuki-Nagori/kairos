/** 图表数据计算：不依赖 Canvas、主题或运行时环境。 */

interface ChartSeriesData {
  values: number[];
}

interface ChartRange {
  min: number;
  max: number;
}

/** 计算曲线值域；无有效数据时返回 null。 */
export function chartRange(series: ChartSeriesData[]): ChartRange | null {
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

/** min-max 抽样：每桶保留最小/最大值。 */
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
      const value = values[index]!;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
    out.push(min, max);
  }
  return out;
}
