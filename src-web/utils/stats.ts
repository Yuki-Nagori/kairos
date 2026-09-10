/** 数值统计助手：循环实现，大数组（10⁶ 量级结果场）不受函数参数栈上限约束。 */

interface MinMax {
  min: number;
  max: number;
}

/**
 * 线性扫描求最小/最大值。
 *
 * 与 `Math.min(...spread)` 的差异：spread 在数组超过参数栈上限（约 10⁵）
 * 时抛 RangeError，本实现任意长度安全；NaN 比较恒为 false，会被跳过
 * （Math.min 则会污染结果为 NaN）——结果场不应含 NaN，语义差异无实害。
 * 空数组返回 { min: Infinity, max: -Infinity }，与 Math.min/max 空参一致。
 */
export function minMax(values: ArrayLike<number>): MinMax {
  let min = Infinity;
  let max = -Infinity;
  for (let index = 0; index < values.length; index += 1) {
    // 不变量：index < length
    const value = values[index]!;
    if (value < min) {
      min = value;
    }
    if (value > max) {
      max = value;
    }
  }
  return { min, max };
}
