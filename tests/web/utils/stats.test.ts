import { describe, expect, it } from "vitest";
import { minMax, quickselect } from "../../../src-web/utils/stats";

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

describe("quickselect", () => {
  it("返回第 k 小（k 从 0 计）", () => {
    expect(quickselect([3, 1, 4, 1, 5], 0)).toBe(1);
    expect(quickselect([3, 1, 4, 1, 5], 2)).toBe(3);
    expect(quickselect([3, 1, 4, 1, 5], 4)).toBe(5);
  });

  it("偶数长度取中间序位（与图例 mid 语义一致）", () => {
    expect(quickselect([9, 2, 7, 4], 2)).toBe(7);
  });

  it("重复值不干扰分区", () => {
    expect(quickselect([2, 2, 2, 2], 1)).toBe(2);
    expect(quickselect([5, 1, 5, 1, 5], 2)).toBe(5);
  });

  it("单元素与空数组", () => {
    expect(quickselect([7], 0)).toBe(7);
    expect(quickselect([], 0)).toBe(0);
  });

  it("与全量排序的中位数一致（1000 个确定性样本对拍）", () => {
    let state = 0x5eed_1234;
    const values: number[] = [];
    for (let i = 0; i < 1000; i += 1) {
      state = (state * 1664525 + 1013904223) >>> 0;
      values.push((state >>> 8) % 10_000);
    }
    const k = 500;
    const expected = [...values].sort((a, b) => a - b)[k];
    expect(quickselect([...values], k)).toBe(expected);
  });

  it("10⁶ 元素量级可用（预算量级冒烟）", () => {
    const values = new Array(1_000_000).fill(1);
    values[123_456] = 42;
    expect(quickselect(values, 500_000)).toBe(1);
  });
});
