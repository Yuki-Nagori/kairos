/**
 * 数值与工程单位格式化：统一有效位与单位口径。
 *
 * 此前各面板各自写 `toFixed` / `toExponential`（45 处、散落 20 个文件），
 * 同一个量在不同面板的位数、单位与占位符不一致；这里收敛成三条：
 * 定点位数、有效位数（含极小/极大值的科学计数）、工程长度换档。
 * 非有限值一律给占位符——NaN / Infinity 漏到界面上是 bug，不是显示问题。
 */

/** 缺少可用数值时的占位符（面板统一用它，避免出现 "NaN" 或空白）。 */
export const NOT_AVAILABLE = "—";

/** 定点位数（固定小数位）。 */
export function fixed(value: number, digits: number, placeholder = NOT_AVAILABLE): string {
  return Number.isFinite(value) ? value.toFixed(digits) : placeholder;
}

/**
 * 有效位数：绝对值在 [1e-3, 1e5) 内用定点，超出用科学计数。
 * 用于量级跨度大的场值（温度 300 K 与残差 1e-9 同屏）。
 */
export function significant(value: number, digits = 3, placeholder = NOT_AVAILABLE): string {
  if (!Number.isFinite(value)) {
    return placeholder;
  }
  if (value === 0) {
    return (0).toFixed(digits);
  }
  const magnitude = Math.abs(value);
  return magnitude >= 1e-3 && magnitude < 1e5 ? value.toFixed(digits) : value.toExponential(digits);
}

/** 工程长度：按量级在 µm / mm / m 之间换档（几何尺寸跨三个量级）。 */
export function lengthFromMm(mm: number, digits = 3, placeholder = NOT_AVAILABLE): string {
  if (!Number.isFinite(mm)) {
    return placeholder;
  }
  const abs = Math.abs(mm);
  if (abs < 1) {
    return `${(mm * 1000).toFixed(digits)} µm`;
  }
  if (abs >= 1000) {
    return `${(mm / 1000).toFixed(digits)} m`;
  }
  return `${mm.toFixed(digits)} mm`;
}
