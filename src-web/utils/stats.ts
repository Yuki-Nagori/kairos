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

/**
 * 就地 quickselect：返回副本第 k 小（均值 O(n)，最坏 O(n²)——图例中值
 * 这类一次性统计可接受，替代对 10⁶ 量级结果场的全量 O(n log n) 排序）。
 * 输入数组会被重排；空数组返回 0。Hoare 分区 + 中位枢轴。
 */
export function quickselect(values: number[], k: number): number {
  if (values.length === 0) {
    return 0;
  }
  let left = 0;
  let right = values.length - 1;
  for (;;) {
    if (left === right) {
      return values[left]!;
    }
    const pivot = values[(left + right) >> 1]!;
    let low = left;
    let high = right;
    while (low <= high) {
      while (values[low]! < pivot) {
        low += 1;
      }
      while (values[high]! > pivot) {
        high -= 1;
      }
      if (low <= high) {
        const tmp = values[low]!;
        values[low] = values[high]!;
        values[high] = tmp;
        low += 1;
        high -= 1;
      }
    }
    if (k <= high) {
      right = high;
    } else if (k >= low) {
      left = low;
    } else {
      return values[k]!;
    }
  }
}
