import { describe, expect, it } from "vitest";
import { minMax } from "../../../src-web/lib/stats";

describe("minMax", () => {
  it("常规数组返回两端值", () => {
    expect(minMax([3, 1, 4, 1, 5])).toEqual({ min: 1, max: 5 });
  });

  it("空数组返回 +/- Infinity（与 Math.min/max 空参一致）", () => {
    expect(minMax([])).toEqual({ min: Infinity, max: -Infinity });
  });

  it("单元素返回相同值", () => {
    expect(minMax([7])).toEqual({ min: 7, max: 7 });
  });

  it("跳过 NaN（不污染结果）", () => {
    expect(minMax([2, Number.NaN, 6])).toEqual({ min: 2, max: 6 });
  });

  it("负数与混合小数", () => {
    expect(minMax([-2.5, 0, 1.5])).toEqual({ min: -2.5, max: 1.5 });
  });

  it("10⁶ 元素线性扫描不溢出（预算量级冒烟）", () => {
    const values = new Array(1_000_000).fill(5);
    values[500_000] = -3;
    values[999_999] = 12;
    expect(minMax(values)).toEqual({ min: -3, max: 12 });
  });
});
